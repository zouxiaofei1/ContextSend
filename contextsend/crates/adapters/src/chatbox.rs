//! ChatBox 适配器：经 Chrome DevTools Protocol(CDP) 注入渲染进程写 IndexedDB。
//!
//! ChatBox 桌面端是 Electron，会话存进 Chromium IndexedDB（localforage 实例
//! `chatboxstore` / objectStore `keyvaluepairs`）：
//!
//! - 会话本体：key=`session:<id>`，value=`JSON.stringify(Session)`
//!   （`Session = {id, type:'chat', name, messages:[Message]}`，
//!   `Message = {id, role, contentParts:[{type:'text',text}], timestamp(ms)}`）。
//! - 侧栏列表：key=`chat-sessions-list`，value=`JSON.stringify([{id,name,type}])`，
//!   决定哪些会话显示在侧栏。仅写 `session:<id>` 不写列表则不会出现在侧栏。
//!
//! 外部进程无法可靠直写 IndexedDB（LevelDB 块 + localforage 序列化），但带
//! `--remote-debugging-port=9222` 启动 ChatBox 后，可经 CDP 连进**渲染进程**用页面
//! 自己的 `indexedDB` 写入，再 `location.reload()` 让侧栏出现新会话（UI 由内存
//! store 驱动，写盘后需刷新）。
//!
//! 注意：ChatBox 单实例锁——已在运行（且没开端口）时再带 flag 启动只会把参数转发给
//! 旧实例后退出。故自动拉起仅在 ChatBox **未运行**时有效；否则需用户先完全退出。

use std::time::Duration;

use cs_core::{ChatMessage, Conversation, Role};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::AdapterError;

/// CDP 调试端口（与启动 ChatBox 时的 `--remote-debugging-port` 一致）。
const DEBUG_PORT: u16 = 9222;
/// DevTools HTTP/WS 监听地址。
const DEBUG_HOST: &str = "127.0.0.1";

/// 把 cs-core 的 [`Role`] 映射为 ChatBox/OpenAI 的角色字符串。
fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

/// 把 ChatBox/OpenAI 的角色字符串映射回 cs-core 的 [`Role`]，未知值按 user。
fn parse_role(s: &str) -> Role {
    match s {
        "system" => Role::System,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        _ => Role::User,
    }
}

/// 为会话取一个展示标题：优先 `title`，否则取首条非空消息前若干字，再退到默认。
fn session_name(convo: &Conversation) -> String {
    if let Some(t) = convo
        .title
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return t.to_string();
    }
    for m in &convo.messages {
        let text = m.text();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let snippet: String = trimmed.chars().take(20).collect();
            return snippet;
        }
    }
    "来自 ContextSend".to_string()
}

/// 对一段原始 HTTP/1.1 响应取 body（`\r\n\r\n` 之后的部分）。
fn http_body(raw: &str) -> Option<&str> {
    raw.split_once("\r\n\r\n").map(|(_, body)| body)
}

/// 从响应头里解析 `Content-Length`（字节数）。
fn content_length(head: &str) -> Option<usize> {
    head.lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        })
        .and_then(|(_, v)| v.trim().parse().ok())
}

/// 向 DevTools HTTP 端点发一个极简 GET，返回响应 body。
///
/// 不引入 HTTP 客户端依赖：DevTools 的 `/json*` 响应短小，裸 TCP 足够。
/// 注意 Chrome 116 会忽略 `Connection: close` 保持 keep-alive，故**不能**用
/// `read_to_end`（会一直阻塞到超时）。改为读到响应头后按 `Content-Length` 收满
/// body 即停；整体加超时兜底。
async fn devtools_get(path: &str) -> Result<String, AdapterError> {
    let addr = format!("{DEBUG_HOST}:{DEBUG_PORT}");
    let fut = async {
        let mut stream = TcpStream::connect(&addr).await.map_err(|e| {
            AdapterError::ChatBox(format!("连接 ChatBox 调试端口 {DEBUG_PORT} 失败: {e}"))
        })?;
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {DEBUG_HOST}:{DEBUG_PORT}\r\nAccept: application/json\r\n\r\n"
        );
        stream.write_all(req.as_bytes()).await?;

        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            // 已收齐响应头 + Content-Length 指定的 body 就停（不等服务端关连接）。
            if let Some(headers_end) = find_headers_end(&buf) {
                let head = String::from_utf8_lossy(&buf[..headers_end]);
                let body_start = headers_end + 4;
                match content_length(&head) {
                    Some(len) if buf.len() - body_start >= len => break,
                    None => break, // 无 Content-Length：已拿到头即视为读完
                    _ => {}
                }
            }
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break; // 服务端真的关了
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        let raw = String::from_utf8_lossy(&buf);
        http_body(&raw)
            .map(|b| b.to_string())
            .ok_or_else(|| AdapterError::ChatBox("DevTools 响应格式异常（无 body）".into()))
    };

    tokio::time::timeout(Duration::from_secs(5), fut)
        .await
        .map_err(|_| AdapterError::ChatBox(format!("DevTools {path} 请求超时")))?
}

/// 在缓冲区里找 `\r\n\r\n` 的起始偏移（响应头结束位置）。
fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// 调试端口是否就绪（能取到 `/json/version`）。
async fn debugger_ready() -> bool {
    devtools_get("/json/version").await.is_ok()
}

/// 取一个可注入的 page target 的 `webSocketDebuggerUrl`。
async fn page_target_ws() -> Result<String, AdapterError> {
    let body = devtools_get("/json").await?;
    let targets: Value = serde_json::from_str(&body)?;
    let arr = targets
        .as_array()
        .ok_or_else(|| AdapterError::ChatBox("DevTools /json 非数组".into()))?;
    arr.iter()
        .find(|t| t.get("type").and_then(|v| v.as_str()) == Some("page"))
        .and_then(|t| t.get("webSocketDebuggerUrl").and_then(|v| v.as_str()))
        .map(String::from)
        .ok_or_else(|| {
            AdapterError::ChatBox("未找到可注入的 page target（ChatBox 窗口未就绪？）".into())
        })
}

/// 在渲染进程里执行一段 JS（`Runtime.evaluate`），返回 `result.value`。
///
/// `await_promise=true` 时等待表达式返回的 Promise resolve。
async fn cdp_evaluate(
    ws_url: &str,
    expression: &str,
    await_promise: bool,
) -> Result<Value, AdapterError> {
    let (mut ws, _) = tokio_tungstenite::connect_async(ws_url)
        .await
        .map_err(|e| AdapterError::ChatBox(format!("CDP WebSocket 连接失败: {e}")))?;

    // 先启用 Runtime 域。
    let enable = serde_json::json!({ "id": 1, "method": "Runtime.enable" });
    ws.send(WsMessage::Text(enable.to_string()))
        .await
        .map_err(|e| AdapterError::ChatBox(format!("CDP 发送失败: {e}")))?;

    let eval = serde_json::json!({
        "id": 2,
        "method": "Runtime.evaluate",
        "params": {
            "expression": expression,
            "awaitPromise": await_promise,
            "returnByValue": true,
        }
    });
    ws.send(WsMessage::Text(eval.to_string()))
        .await
        .map_err(|e| AdapterError::ChatBox(format!("CDP 发送失败: {e}")))?;

    // 读消息直到拿到 id==2 的响应（跳过事件与 enable 的 ack）。
    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| AdapterError::ChatBox(format!("CDP 接收失败: {e}")))?;
        let text = match msg {
            WsMessage::Text(t) => t,
            WsMessage::Close(_) => break,
            _ => continue,
        };
        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("id").and_then(|i| i.as_u64()) != Some(2) {
            continue;
        }
        // 注入脚本本身抛异常。
        if let Some(exc) = v.get("result").and_then(|r| r.get("exceptionDetails")) {
            return Err(AdapterError::ChatBox(format!("注入脚本异常: {exc}")));
        }
        let value = v
            .pointer("/result/result/value")
            .cloned()
            .unwrap_or(Value::Null);
        let _ = ws.close(None).await;
        return Ok(value);
    }
    Err(AdapterError::ChatBox("CDP 未返回 evaluate 结果".into()))
}

/// 构造写入会话 + 更新侧栏列表的注入脚本。
///
/// `payload` 形如 `{"name":..., "messages":[{"role":..,"text":..}]}`，
/// `session_id` 为新会话 id（由 Rust 生成）。脚本在页面 main world 用页面自身的
/// `indexedDB` 写 `chatboxstore`，与 ChatBox 的 localforage 存储格式一致。
fn build_import_script(session_id: &str, payload: &Value) -> String {
    // payload / id 以 JSON 字面量嵌入（JSON 是合法 JS 表达式）。
    format!(
        r#"(async () => {{
  const payload = {payload};
  const sid = {sid};
  const now = Date.now();
  const session = {{
    id: sid,
    type: 'chat',
    name: payload.name,
    messages: payload.messages.map((m, i) => ({{
      id: sid + '-' + i,
      role: m.role,
      contentParts: [{{ type: 'text', text: m.text }}],
      timestamp: now + i,
    }})),
  }};
  const db = await new Promise((res, rej) => {{
    const r = indexedDB.open('chatboxstore');
    r.onsuccess = () => res(r.result);
    r.onerror = () => rej(r.error);
  }});
  const tx = db.transaction('keyvaluepairs', 'readwrite');
  const os = tx.objectStore('keyvaluepairs');
  const get = (k) => new Promise((res, rej) => {{ const g = os.get(k); g.onsuccess = () => res(g.result); g.onerror = () => rej(g.error); }});
  const put = (k, v) => new Promise((res, rej) => {{ const p = os.put(v, k); p.onsuccess = () => res(); p.onerror = () => rej(p.error); }});
  await put('session:' + sid, JSON.stringify(session));
  let list = [];
  try {{ const raw = await get('chat-sessions-list'); if (raw) list = JSON.parse(raw); }} catch (e) {{ list = []; }}
  if (!Array.isArray(list)) list = [];
  list.push({{ id: sid, name: payload.name, type: 'chat' }});
  await put('chat-sessions-list', JSON.stringify(list));
  await new Promise((res, rej) => {{ tx.oncomplete = () => res(); tx.onerror = () => rej(tx.error); }});
  return {{ ok: true, id: sid, total: list.length }};
}})()"#,
        payload = payload,
        sid = serde_json::Value::String(session_id.to_string()),
    )
}

/// 把一段对话导入 ChatBox：经 CDP 写 IndexedDB 并刷新侧栏，返回新会话 id。
///
/// 前置条件：ChatBox 已带 `--remote-debugging-port=9222` 启动。端口不可达时返回
/// 明确错误，提示用户用带该 flag 的方式启动 ChatBox（单实例锁下需先完全退出再启动）。
pub async fn import_to_chatbox(convo: &Conversation) -> Result<String, AdapterError> {
    log::debug!(
        "ChatBox 导入开始: messages={} (CDP 端口 {DEBUG_PORT})",
        convo.messages.len()
    );
    if !debugger_ready().await {
        log::warn!("ChatBox 调试端口 {DEBUG_PORT} 不可达，导入中止");
        return Err(AdapterError::ChatBox(format!(
            "ChatBox 调试端口 {DEBUG_PORT} 不可达。请先完全退出 ChatBox，再以 \
             `--remote-debugging-port={DEBUG_PORT}` 启动后重试。"
        )));
    }

    let ws_url = page_target_ws().await?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let messages: Vec<Value> = convo
        .messages
        .iter()
        .map(|m| serde_json::json!({ "role": role_str(m.role), "text": m.text() }))
        .collect();
    let payload = serde_json::json!({
        "name": session_name(convo),
        "messages": messages,
    });

    let script = build_import_script(&session_id, &payload);
    let result = cdp_evaluate(&ws_url, &script, true).await?;

    if result.get("ok").and_then(|b| b.as_bool()) != Some(true) {
        return Err(AdapterError::ChatBox(format!(
            "写入未确认成功，注入返回: {result}"
        )));
    }

    // 刷新渲染进程，让 ChatBox 从 IndexedDB 重读 → 侧栏出现新会话。
    // 页面随即导航，evaluate 可能无回执，故不等待 Promise、忽略其结果。
    let _ = cdp_evaluate(&ws_url, "location.reload()", false).await;

    // 给重载留一点时间（best-effort，不影响返回）。
    tokio::time::sleep(Duration::from_millis(50)).await;

    log::info!("ChatBox 导入完成: session_id={session_id}");
    Ok(session_id)
}

/// 读取脚本：遍历 `chatboxstore` 里所有 `session:` 键，把每个会话**按话题(thread)
/// 拆开**返回。
///
/// ChatBox 一个 session 含多个话题：当前活跃话题在 `session.messages`（名字在
/// `session.threadName`），点"新话题"后旧话题归档进 `session.threads[]`（每个
/// `{id,name,messages}`）。只读 `messages` 会漏掉所有历史话题，故这里把
/// `threads[]` 的每个 + 当前 `messages` 都各自展开成一条 `{id,name,messages}`。
///
/// 用页面自身的 localforage 存储格式解码（value 是 `JSON.stringify(Session)`），
/// 不碰底层 LevelDB 编码，故稳定可靠。`contentParts` 原样回传，由 Rust 侧拼成文本。
const READ_SCRIPT: &str = r#"(async () => {
  const db = await new Promise((res, rej) => {
    const r = indexedDB.open('chatboxstore');
    r.onsuccess = () => res(r.result);
    r.onerror = () => rej(r.error);
  });
  if (!db.objectStoreNames.contains('keyvaluepairs')) return [];
  const os = db.transaction('keyvaluepairs', 'readonly').objectStore('keyvaluepairs');
  const entries = await new Promise((res, rej) => {
    const out = [];
    const cur = os.openCursor();
    cur.onsuccess = (e) => {
      const c = e.target.result;
      if (!c) return res(out);
      const key = String(c.key);
      if (key.startsWith('session:')) out.push([key, c.value]);
      c.continue();
    };
    cur.onerror = () => rej(cur.error);
  });
  const slim = (m) => ({
    role: m && m.role ? String(m.role) : 'user',
    contentParts: Array.isArray(m && m.contentParts) ? m.contentParts : [],
  });
  const out = [];
  for (const [key, raw] of entries) {
    let s;
    try { s = typeof raw === 'string' ? JSON.parse(raw) : raw; } catch (e) { continue; }
    if (!s || typeof s !== 'object') continue;
    if (s.type && s.type !== 'chat') continue; // 仅对话类，跳过图片等
    const sid = key.slice('session:'.length);
    const sessionName = s.name || '';
    // 已归档的历史话题：各成一条。
    if (Array.isArray(s.threads)) {
      for (const th of s.threads) {
        if (!th || !Array.isArray(th.messages)) continue;
        out.push({ id: sid + ':' + (th.id || out.length), name: th.name || sessionName, messages: th.messages.map(slim) });
      }
    }
    // 当前活跃话题：名字优先 threadName，回退 session 名。
    const curMsgs = Array.isArray(s.messages) ? s.messages.map(slim) : [];
    if (curMsgs.length > 0) {
      out.push({ id: sid, name: s.threadName || sessionName, messages: curMsgs });
    }
  }
  return out;
})()"#;

/// 把 ChatBox 的一条 `contentParts` 数组拼成纯文本（文本块直拼，其它块转占位符）。
fn content_parts_to_text(parts: &Value) -> String {
    let Some(arr) = parts.as_array() else {
        return String::new();
    };
    let mut out = String::new();
    for part in arr {
        match part.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                }
            }
            Some("reasoning") => {
                // 推理块的文本字段也叫 text。
                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                }
            }
            Some("image") => out.push_str("[图片]"),
            Some("tool-call") => out.push_str("[工具调用]"),
            _ => {}
        }
    }
    out
}

/// 把读取脚本返回的精简会话 JSON 数组映射为 [`Conversation`] 列表。
fn sessions_json_to_conversations(value: &Value) -> Vec<Conversation> {
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    let mut convos = Vec::new();
    for s in arr {
        let mut convo = Conversation::new();
        convo.title = s
            .get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());

        if let Some(msgs) = s.get("messages").and_then(|m| m.as_array()) {
            for m in msgs {
                let role = m
                    .get("role")
                    .and_then(|r| r.as_str())
                    .map(parse_role)
                    .unwrap_or(Role::User);
                let text = m
                    .get("contentParts")
                    .map(content_parts_to_text)
                    .unwrap_or_default();
                convo.messages.push(ChatMessage::new(role, text));
            }
        }
        convos.push(convo);
    }
    convos
}

/// 读取 ChatBox 里的所有对话会话（导出 / 匹配方向）。
///
/// 前置条件同导入：ChatBox 已带 `--remote-debugging-port=9222` 启动。端口不可达时
/// 返回 [`AdapterError::ChatBox`]，由调用方决定是否忽略（匹配遍历会跳过失败的适配器）。
pub async fn list_chatbox_conversations() -> Result<Vec<Conversation>, AdapterError> {
    if !debugger_ready().await {
        log::debug!("ChatBox 调试端口 {DEBUG_PORT} 不可达，跳过读取");
        return Err(AdapterError::ChatBox(format!(
            "ChatBox 调试端口 {DEBUG_PORT} 不可达（读取需 ChatBox 带 \
             `--remote-debugging-port={DEBUG_PORT}` 运行）。"
        )));
    }
    let ws_url = page_target_ws().await?;
    let result = cdp_evaluate(&ws_url, READ_SCRIPT, true).await?;
    let convos = sessions_json_to_conversations(&result);
    log::debug!("ChatBox 读取完成: 共 {} 个会话(话题)", convos.len());
    Ok(convos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::ChatMessage;

    #[test]
    fn role_maps_to_chatbox_strings() {
        assert_eq!(role_str(Role::User), "user");
        assert_eq!(role_str(Role::Assistant), "assistant");
        assert_eq!(role_str(Role::System), "system");
        assert_eq!(role_str(Role::Tool), "tool");
    }

    #[test]
    fn parse_role_maps_known_and_unknown() {
        assert_eq!(parse_role("system"), Role::System);
        assert_eq!(parse_role("assistant"), Role::Assistant);
        assert_eq!(parse_role("tool"), Role::Tool);
        assert_eq!(parse_role("user"), Role::User);
        assert_eq!(parse_role("anything-else"), Role::User);
    }

    #[test]
    fn content_parts_join_text_and_placeholders() {
        let parts = serde_json::json!([
            { "type": "text", "text": "你好" },
            { "type": "image", "storageKey": "x" },
            { "type": "text", "text": "世界" },
        ]);
        assert_eq!(content_parts_to_text(&parts), "你好[图片]世界");
    }

    #[test]
    fn content_parts_empty_is_empty_string() {
        assert_eq!(content_parts_to_text(&serde_json::json!([])), "");
        assert_eq!(content_parts_to_text(&serde_json::Value::Null), "");
    }

    #[test]
    fn sessions_json_maps_to_conversations() {
        let value = serde_json::json!([
            {
                "id": "s1",
                "name": "天气对话",
                "messages": [
                    { "role": "user", "contentParts": [{ "type": "text", "text": "今天天气如何" }] },
                    { "role": "assistant", "contentParts": [{ "type": "text", "text": "晴" }] },
                ]
            }
        ]);
        let convos = sessions_json_to_conversations(&value);
        assert_eq!(convos.len(), 1);
        assert_eq!(convos[0].title.as_deref(), Some("天气对话"));
        assert_eq!(convos[0].messages.len(), 2);
        assert_eq!(convos[0].messages[0].role, Role::User);
        assert_eq!(convos[0].messages[0].text(), "今天天气如何");
        assert_eq!(convos[0].messages[1].role, Role::Assistant);
    }

    #[test]
    fn sessions_json_non_array_yields_empty() {
        assert!(sessions_json_to_conversations(&serde_json::json!({})).is_empty());
        assert!(sessions_json_to_conversations(&serde_json::Value::Null).is_empty());
    }

    #[test]
    fn read_script_scopes_to_session_prefix() {
        assert!(READ_SCRIPT.contains("chatboxstore"));
        assert!(READ_SCRIPT.contains("session:"));
        assert!(READ_SCRIPT.contains("contentParts"));
        // 必须展开历史话题，而非只读当前 messages。
        assert!(READ_SCRIPT.contains("s.threads"));
        assert!(READ_SCRIPT.contains("threadName"));
    }

    #[test]
    fn multiple_threads_map_to_separate_conversations() {
        // 模拟读取脚本对一个含历史话题的 session 的输出：每个话题各一条。
        let value = serde_json::json!([
            { "id": "sid:t1", "name": "话题一", "messages": [
                { "role": "user", "contentParts": [{ "type": "text", "text": "甲" }] }
            ]},
            { "id": "sid:t2", "name": "话题二", "messages": [
                { "role": "user", "contentParts": [{ "type": "text", "text": "乙" }] }
            ]},
            { "id": "sid", "name": "当前话题", "messages": [
                { "role": "user", "contentParts": [{ "type": "text", "text": "丙" }] }
            ]},
        ]);
        let convos = sessions_json_to_conversations(&value);
        assert_eq!(convos.len(), 3);
        let titles: Vec<_> = convos.iter().filter_map(|c| c.title.clone()).collect();
        assert_eq!(titles, vec!["话题一", "话题二", "当前话题"]);
    }

    #[test]
    fn session_name_prefers_title() {
        let mut c = Conversation::new();
        c.title = Some("我的标题".into());
        c.messages.push(ChatMessage::user("正文很长很长"));
        assert_eq!(session_name(&c), "我的标题");
    }

    #[test]
    fn session_name_falls_back_to_first_message_snippet() {
        let mut c = Conversation::new();
        c.messages
            .push(ChatMessage::user("这是第一条消息的内容用于做标题回退"));
        let name = session_name(&c);
        assert!(name.starts_with("这是第一条消息"));
        assert!(name.chars().count() <= 20);
    }

    #[test]
    fn session_name_default_when_empty() {
        let c = Conversation::new();
        assert_eq!(session_name(&c), "来自 ContextSend");
    }

    #[test]
    fn http_body_splits_on_blank_line() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
        assert_eq!(http_body(raw), Some("{\"a\":1}"));
    }

    #[test]
    fn content_length_parsed_case_insensitive() {
        let head = "HTTP/1.1 200 OK\r\ncontent-length: 42\r\nContent-Type: application/json";
        assert_eq!(content_length(head), Some(42));
        let head2 = "HTTP/1.1 200 OK\r\nContent-Length:   7  ";
        assert_eq!(content_length(head2), Some(7));
        assert_eq!(content_length("HTTP/1.1 200 OK"), None);
    }

    #[test]
    fn find_headers_end_locates_blank_line() {
        let buf = b"AB\r\n\r\nCD";
        assert_eq!(find_headers_end(buf), Some(2));
        assert_eq!(find_headers_end(b"no-blank-line"), None);
    }

    #[test]
    fn import_script_embeds_payload_and_id() {
        let payload = serde_json::json!({
            "name": "测试",
            "messages": [{ "role": "user", "text": "你好" }]
        });
        let script = build_import_script("abc-123", &payload);
        assert!(script.contains("chatboxstore"));
        assert!(script.contains("keyvaluepairs"));
        assert!(script.contains("chat-sessions-list"));
        assert!(script.contains("\"abc-123\""));
        assert!(script.contains("你好"));
        // session 值以 JSON 字符串写入（localforage 兼容）。
        assert!(script.contains("JSON.stringify(session)"));
    }
}
