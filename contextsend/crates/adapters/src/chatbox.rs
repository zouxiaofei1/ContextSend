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

use cs_core::{
    ChatMessage, ContentPart, Conversation, ImageUrl, MessageContent, MessageMetadata, Role,
    TokenUsage,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::AdapterError;

/// CDP 调试端口默认值（与启动 ChatBox 时的 `--remote-debugging-port` 一致）。
/// 可被用户在「设置 → 适配器 → ChatBox」里覆盖。
pub(crate) const DEFAULT_DEBUG_PORT: u16 = 9222;
/// DevTools HTTP/WS 监听地址。
const DEBUG_HOST: &str = "127.0.0.1";

/// 当前生效的 CDP 调试端口：用户覆盖优先，否则 [`DEFAULT_DEBUG_PORT`]。
fn debug_port() -> u16 {
    crate::config::get("chatbox")
        .port
        .unwrap_or(DEFAULT_DEBUG_PORT)
}

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

/// 把一条 [`ChatMessage`] 转成注入脚本期望的 `parts` 数组。
///
/// - 纯文本消息 → 单个 `{type:'text', text}`。
/// - 多模态消息 → 文本块转 `{type:'text',..}`、图像块转 `{type:'image', url}`
///   （`url` 直接传 [`ImageUrl::url`]，可以是 `data:` dataURL 或 `http(s)` 链接，
///   注入脚本会把它经 `setStoreBlob` 落进 ChatBox 的 blob 存储）。
fn message_to_parts(m: &ChatMessage) -> Vec<Value> {
    match &m.content {
        MessageContent::Text(t) => vec![serde_json::json!({ "type": "text", "text": t })],
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|p| match p {
                ContentPart::Text { text } => serde_json::json!({ "type": "text", "text": text }),
                ContentPart::ImageUrl { image_url } => {
                    serde_json::json!({ "type": "image", "url": image_url.url })
                }
            })
            .collect(),
    }
}

/// 把 [`MessageMetadata`] 转成注入脚本期望的 `meta` 对象（camelCase，空字段省略）。
///
/// 与 [`parse_metadata`] 对称：仅写出有值的字段，`usage` 同理逐字段省略空值；
/// 注入脚本据此还原 ChatBox 的 `model`/`aiProvider`/`firstTokenLatency`/
/// `finishReason`/`tokensUsed`/`usage`。
fn metadata_to_json(meta: &MessageMetadata) -> Value {
    let mut obj = serde_json::Map::new();
    if let Some(m) = &meta.model {
        obj.insert("model".into(), Value::String(m.clone()));
    }
    if let Some(p) = &meta.provider {
        obj.insert("provider".into(), Value::String(p.clone()));
    }
    if let Some(l) = meta.first_token_latency_ms {
        obj.insert("firstTokenLatency".into(), Value::from(l));
    }
    if let Some(f) = &meta.finish_reason {
        obj.insert("finishReason".into(), Value::String(f.clone()));
    }
    if let Some(u) = meta.usage.as_ref().filter(|u| !u.is_empty()) {
        let mut um = serde_json::Map::new();
        let mut put = |k: &str, v: Option<u64>| {
            if let Some(v) = v {
                um.insert(k.into(), Value::from(v));
            }
        };
        put("inputTokens", u.input_tokens);
        put("outputTokens", u.output_tokens);
        put("totalTokens", u.total_tokens);
        put("reasoningTokens", u.reasoning_tokens);
        put("cachedInputTokens", u.cached_input_tokens);
        obj.insert("usage".into(), Value::Object(um));
    }
    Value::Object(obj)
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
    let port = debug_port();
    let addr = format!("{DEBUG_HOST}:{port}");
    let fut = async {
        let mut stream = TcpStream::connect(&addr).await.map_err(|e| {
            AdapterError::ChatBox(format!("连接 ChatBox 调试端口 {port} 失败: {e}"))
        })?;
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {DEBUG_HOST}:{port}\r\nAccept: application/json\r\n\r\n"
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
/// `payload` 形如 `{"name":.., "messages":[{"role":.., "parts":[Part], "meta":{..}?}]}`，
/// 其中 `Part` 是 `{"type":"text","text":..}` 或 `{"type":"image","url":<dataURL>}`，
/// `meta`（可选）携带 `model`/`provider`/`firstTokenLatency`/`finishReason`/`usage`，
/// 写回时还原成 ChatBox 自身的消息字段（`model`/`aiProvider`/`firstTokenLatency`/
/// `finishReason`/`tokensUsed`/`usage`），使导入的会话也显示原始模型与用量统计。
/// `session_id` 为新会话 id（由 Rust 生成）。
///
/// 脚本在页面 main world：
/// 1. **先**把每个图片块的 dataURL 经 `electronAPI.invoke('setStoreBlob', key, url)`
///    写入主进程 `chatbox-blobs/`（key 形如 `picture:<uuid>`），消息里换成
///    `{type:'image', storageKey:key}`——这与 ChatBox 自身的图片存储格式一致。
/// 2. **再**用页面自身的 `indexedDB` 写 `chatboxstore` 的 `session:<id>` 与侧栏列表。
///    （图片写盘是异步 IPC，不能放进 IDB 事务内，故分两步。）
fn build_import_script(session_id: &str, payload: &Value) -> String {
    // payload / id 以 JSON 字面量嵌入（JSON 是合法 JS 表达式）。
    format!(
        r#"(async () => {{
  const payload = {payload};
  const sid = {sid};
  const now = Date.now();
  const api = (typeof window !== 'undefined') ? (window.electronAPI || window.platform) : null;
  const newKey = () => 'picture:' + ((crypto && crypto.randomUUID) ? crypto.randomUUID() : (sid + '-img-' + Math.random().toString(36).slice(2)));
  // 把一条消息的 parts 转成 ChatBox 的 contentParts；图片先写 blob 再引用 storageKey。
  const buildParts = async (parts) => {{
    const out = [];
    for (const p of (Array.isArray(parts) ? parts : [])) {{
      if (p && p.type === 'image' && p.url) {{
        let stored = false;
        if (api && api.invoke) {{
          try {{ const key = newKey(); await api.invoke('setStoreBlob', key, p.url); out.push({{ type: 'image', storageKey: key }}); stored = true; }} catch (e) {{ stored = false; }}
        }}
        if (!stored) out.push({{ type: 'text', text: '[图片]' }}); // 无写盘桥时降级
      }} else if (p && p.type === 'text') {{
        out.push({{ type: 'text', text: String(p.text || '') }});
      }}
    }}
    if (out.length === 0) out.push({{ type: 'text', text: '' }});
    return out;
  }};
  // 把 meta 还原成 ChatBox 的消息字段（仅在确有值时写，避免污染 user 消息）。
  const applyMeta = (msg, meta) => {{
    if (!meta || typeof meta !== 'object') return;
    if (meta.model != null) msg.model = meta.model;
    if (meta.provider != null) msg.aiProvider = meta.provider;
    if (typeof meta.firstTokenLatency === 'number') msg.firstTokenLatency = meta.firstTokenLatency;
    if (meta.finishReason != null) msg.finishReason = meta.finishReason;
    const u = meta.usage;
    if (u && typeof u === 'object') {{
      msg.usage = u;
      if (typeof u.totalTokens === 'number') msg.tokensUsed = u.totalTokens;
    }}
  }};
  const messages = [];
  for (let i = 0; i < payload.messages.length; i++) {{
    const m = payload.messages[i];
    const msg = {{ id: sid + '-' + i, role: m.role, contentParts: await buildParts(m.parts), timestamp: now + i }};
    applyMeta(msg, m.meta);
    messages.push(msg);
  }}
  const session = {{ id: sid, type: 'chat', name: payload.name, messages }};
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
    let port = debug_port();
    log::debug!(
        "ChatBox 导入开始: messages={} (CDP 端口 {port})",
        convo.messages.len()
    );
    if !debugger_ready().await {
        log::warn!("ChatBox 调试端口 {port} 不可达，导入中止");
        return Err(AdapterError::ChatBox(format!(
            "ChatBox 调试端口 {port} 不可达。请先完全退出 ChatBox，再以 \
             `--remote-debugging-port={port}` 启动后重试。"
        )));
    }

    let ws_url = page_target_ws().await?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let messages: Vec<Value> = convo
        .messages
        .iter()
        .map(|m| {
            let mut obj = serde_json::json!({ "role": role_str(m.role), "parts": message_to_parts(m) });
            // 仅在有非空元数据时附带 meta，写回 ChatBox 的 model/usage/延迟等字段。
            if let Some(meta) = m.metadata.as_ref().filter(|md| !md.is_empty()) {
                obj["meta"] = metadata_to_json(meta);
            }
            obj
        })
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
/// 拆开**返回，并把图片块的 `storageKey` 解析成真正的图片数据。
///
/// ChatBox 一个 session 含多个话题：当前活跃话题在 `session.messages`（名字在
/// `session.threadName`），点"新话题"后旧话题归档进 `session.threads[]`（每个
/// `{id,name,messages}`）。只读 `messages` 会漏掉所有历史话题，故这里把
/// `threads[]` 的每个 + 当前 `messages` 都各自展开成一条 `{id,name,messages}`。
///
/// **图片处理**：ChatBox 的图片块是 `{type:'image', storageKey:'picture:<uuid>'}`，
/// 真正的字节（`data:image/...;base64,` dataURL）存在主进程文件系统的 `chatbox-blobs/`
/// 目录里，渲染进程经 `electronAPI.invoke('getStoreBlob', storageKey)` 读取。脚本在
/// 渲染进程内把每个图片块就地解析为 `{type:'image', url:<dataURL>}`（带缓存，避免对
/// 同一 key 重复 IPC），解析失败则保留 `storageKey` 由 Rust 侧降级为占位符。
///
/// **元数据**：ChatBox 在 assistant 消息上记录 `model` / `aiProvider` /
/// `firstTokenLatency`(ms) / `finishReason` / `usage`(input/output/total/reasoning/
/// cached token)等。脚本把这些归一成每条消息的 `meta` 对象一并回传，由 Rust 侧
/// [`parse_metadata`] 填入 [`MessageMetadata`]，随对话传输 / 展示。
///
/// 用页面自身的 localforage 存储格式解码（value 是 `JSON.stringify(Session)`），
/// 不碰底层 LevelDB 编码，故稳定可靠。
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
  // 图片字节读取桥 + 缓存（同一 storageKey 只取一次）。
  const blobCache = new Map();
  const api = (typeof window !== 'undefined') ? (window.electronAPI || window.platform) : null;
  const resolveBlob = async (key) => {
    if (blobCache.has(key)) return blobCache.get(key);
    let url = null;
    try { if (api && api.invoke) url = await api.invoke('getStoreBlob', key); } catch (e) { url = null; }
    blobCache.set(key, url);
    return url;
  };
  // 把一条消息精简为 {role, contentParts, meta}，并把 image 块的 storageKey 解析成 url。
  const slim = async (m) => {
    const role = m && m.role ? String(m.role) : 'user';
    const rawParts = Array.isArray(m && m.contentParts) ? m.contentParts : [];
    const parts = [];
    for (const p of rawParts) {
      if (p && p.type === 'image' && p.storageKey) {
        const url = await resolveBlob(String(p.storageKey));
        parts.push(url ? { type: 'image', url } : { type: 'image', storageKey: String(p.storageKey) });
      } else {
        parts.push(p);
      }
    }
    // 生成元数据：model / provider / 首字延迟 / finishReason / token 用量。
    // 仅在确有字段时附带 meta，避免给 user 消息塞空对象。
    let meta = null;
    if (m && (m.model || m.aiProvider || m.usage || m.firstTokenLatency != null || m.finishReason || m.tokensUsed != null)) {
      const u = (m.usage && typeof m.usage === 'object') ? m.usage : null;
      meta = {
        model: m.model != null ? String(m.model) : null,
        provider: m.aiProvider != null ? String(m.aiProvider) : null,
        firstTokenLatency: (typeof m.firstTokenLatency === 'number') ? m.firstTokenLatency : null,
        finishReason: m.finishReason != null ? String(m.finishReason) : null,
        usage: u ? {
          inputTokens: (typeof u.inputTokens === 'number') ? u.inputTokens : null,
          outputTokens: (typeof u.outputTokens === 'number') ? u.outputTokens : null,
          totalTokens: (typeof u.totalTokens === 'number') ? u.totalTokens : ((typeof m.tokensUsed === 'number') ? m.tokensUsed : null),
          reasoningTokens: (typeof u.reasoningTokens === 'number') ? u.reasoningTokens : null,
          cachedInputTokens: (typeof u.cachedInputTokens === 'number') ? u.cachedInputTokens : null,
        } : ((typeof m.tokensUsed === 'number') ? { totalTokens: m.tokensUsed } : null),
      };
    }
    return { role, contentParts: parts, meta };
  };
  const slimAll = async (msgs) => {
    const r = [];
    for (const m of msgs) r.push(await slim(m));
    return r;
  };
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
        out.push({ id: sid + ':' + (th.id || out.length), name: th.name || sessionName, messages: await slimAll(th.messages) });
      }
    }
    // 当前活跃话题：名字优先 threadName，回退 session 名。
    const curMsgs = Array.isArray(s.messages) ? await slimAll(s.messages) : [];
    if (curMsgs.length > 0) {
      out.push({ id: sid, name: s.threadName || sessionName, messages: curMsgs });
    }
  }
  return out;
})()"#;

/// 把 ChatBox 的一条 `contentParts` 数组转成 cs-core 的 [`MessageContent`]。
///
/// - `text` / `reasoning` 块 → 文本，相邻文本会合并到同一个文本块。
/// - `image` 块：读取脚本已把 `storageKey` 解析成 dataURL 放进 `url` 字段 →
///   转成 [`ContentPart::ImageUrl`]（真正的图片得以保留）。解析失败（无 `url`，
///   仅剩 `storageKey`）则降级为 `[图片]` 文本占位符。
/// - `tool-call` 等其它块 → `[工具调用]` 文本占位符。
///
/// 若最终只有文本、没有图片，返回 [`MessageContent::Text`]（紧凑形态）；
/// 含至少一个图片块时返回 [`MessageContent::Parts`]（多模态形态）。
fn content_parts_to_content(parts: &Value) -> MessageContent {
    let Some(arr) = parts.as_array() else {
        return MessageContent::Text(String::new());
    };
    let mut result: Vec<ContentPart> = Vec::new();
    // 把一段文本追加到结果：与上一个文本块合并，避免产生大量碎块。
    let push_text = |result: &mut Vec<ContentPart>, t: &str| {
        if t.is_empty() {
            return;
        }
        if let Some(ContentPart::Text { text }) = result.last_mut() {
            text.push_str(t);
        } else {
            result.push(ContentPart::Text { text: t.to_string() });
        }
    };
    for part in arr {
        match part.get("type").and_then(|t| t.as_str()) {
            Some("text") | Some("reasoning") => {
                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                    push_text(&mut result, t);
                }
            }
            Some("image") => {
                // 读取脚本把字节解析进 url；拿到则保留为真正的图片块。
                if let Some(url) = part.get("url").and_then(|u| u.as_str()).filter(|u| !u.is_empty())
                {
                    result.push(ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: url.to_string(),
                            detail: None,
                        },
                    });
                } else {
                    // 未能取到图片字节（如 ChatBox 未运行该桥）→ 占位符。
                    push_text(&mut result, "[图片]");
                }
            }
            Some("tool-call") => push_text(&mut result, "[工具调用]"),
            _ => {}
        }
    }

    // 无图片 → 紧凑文本形态；含图片 → 多模态 Parts 形态。
    let has_image = result
        .iter()
        .any(|p| matches!(p, ContentPart::ImageUrl { .. }));
    if has_image {
        MessageContent::Parts(result)
    } else {
        let text = result
            .into_iter()
            .map(|p| match p {
                ContentPart::Text { text } => text,
                ContentPart::ImageUrl { .. } => String::new(),
            })
            .collect::<String>();
        MessageContent::Text(text)
    }
}

/// 从读取脚本回传的 `meta` 对象解析出 [`MessageMetadata`]；空 / 缺失返回 `None`。
///
/// 脚本已把字段名归一为 camelCase（`model` / `provider` / `firstTokenLatency` /
/// `finishReason` / `usage.{input,output,total,reasoning,cachedInput}Tokens`），
/// 这里逐字段取值并跳过空串 / 非数值，最终全空则不附带元数据。
fn parse_metadata(meta: &Value) -> Option<MessageMetadata> {
    if !meta.is_object() {
        return None;
    }
    let s = |k: &str| {
        meta.get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let usage = meta.get("usage").filter(|u| u.is_object()).map(|u| {
        let n = |k: &str| u.get(k).and_then(serde_json::Value::as_u64);
        TokenUsage {
            input_tokens: n("inputTokens"),
            output_tokens: n("outputTokens"),
            total_tokens: n("totalTokens"),
            reasoning_tokens: n("reasoningTokens"),
            cached_input_tokens: n("cachedInputTokens"),
        }
    });
    let metadata = MessageMetadata {
        model: s("model"),
        provider: s("provider"),
        usage: usage.filter(|u| !u.is_empty()),
        first_token_latency_ms: meta.get("firstTokenLatency").and_then(Value::as_u64),
        finish_reason: s("finishReason"),
    };
    (!metadata.is_empty()).then_some(metadata)
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
                let content = m
                    .get("contentParts")
                    .map(content_parts_to_content)
                    .unwrap_or_else(|| MessageContent::Text(String::new()));
                let metadata = m.get("meta").and_then(parse_metadata);
                // 用首条带模型的消息回填会话级 model（便于卡片标题 / 列表展示）。
                if convo.model.is_none() {
                    if let Some(model) = metadata.as_ref().and_then(|md| md.model.clone()) {
                        convo.model = Some(model);
                    }
                }
                convo.messages.push(ChatMessage {
                    role,
                    content,
                    name: None,
                    metadata,
                });
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
    let port = debug_port();
    if !debugger_ready().await {
        log::debug!("ChatBox 调试端口 {port} 不可达，跳过读取");
        return Err(AdapterError::ChatBox(format!(
            "ChatBox 调试端口 {port} 不可达（读取需 ChatBox 带 \
             `--remote-debugging-port={port}` 运行）。"
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
    fn content_parts_text_only_yields_text_content() {
        // 无图片 → 紧凑文本形态，相邻文本合并，未知图片(无 url)降级为占位符。
        let parts = serde_json::json!([
            { "type": "text", "text": "你好" },
            { "type": "image", "storageKey": "x" },
            { "type": "text", "text": "世界" },
        ]);
        let content = content_parts_to_content(&parts);
        assert_eq!(content, MessageContent::Text("你好[图片]世界".into()));
        assert!(!content.has_image());
    }

    #[test]
    fn content_parts_with_resolved_image_yields_multimodal() {
        // 读取脚本已把 storageKey 解析进 url → 应保留为真正的图片块。
        let parts = serde_json::json!([
            { "type": "text", "text": "看这张" },
            { "type": "image", "url": "data:image/png;base64,AAAA" },
        ]);
        let content = content_parts_to_content(&parts);
        assert!(content.has_image());
        match content {
            MessageContent::Parts(p) => {
                assert_eq!(p.len(), 2);
                assert!(matches!(&p[0], ContentPart::Text { text } if text == "看这张"));
                assert!(
                    matches!(&p[1], ContentPart::ImageUrl { image_url } if image_url.url == "data:image/png;base64,AAAA")
                );
            }
            _ => panic!("含图片应为 Parts 形态"),
        }
    }

    #[test]
    fn content_parts_empty_is_empty_text() {
        assert_eq!(
            content_parts_to_content(&serde_json::json!([])),
            MessageContent::Text(String::new())
        );
        assert_eq!(
            content_parts_to_content(&serde_json::Value::Null),
            MessageContent::Text(String::new())
        );
    }

    #[test]
    fn message_to_parts_maps_text_and_image() {
        // 纯文本消息 → 单个 text part。
        let text_msg = ChatMessage::user("你好");
        let parts = message_to_parts(&text_msg);
        assert_eq!(parts, vec![serde_json::json!({ "type": "text", "text": "你好" })]);

        // 多模态消息 → text + image(url) part，url 原样透传给注入脚本。
        let img_msg = ChatMessage {
            role: Role::User,
            content: MessageContent::Parts(vec![
                ContentPart::Text { text: "看图".into() },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "data:image/png;base64,BBBB".into(),
                        detail: None,
                    },
                },
            ]),
            name: None,
            metadata: None,
        };
        let parts = message_to_parts(&img_msg);
        assert_eq!(
            parts,
            vec![
                serde_json::json!({ "type": "text", "text": "看图" }),
                serde_json::json!({ "type": "image", "url": "data:image/png;base64,BBBB" }),
            ]
        );
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
    fn parse_metadata_extracts_model_usage_latency() {
        // 模拟读取脚本回传的 meta（已归一为 camelCase）。
        let meta = serde_json::json!({
            "model": "OpenAI API (gpt-4o)",
            "provider": "openai",
            "firstTokenLatency": 10319,
            "finishReason": "stop",
            "usage": {
                "inputTokens": 15,
                "outputTokens": 415,
                "totalTokens": 430,
                "reasoningTokens": 0,
                "cachedInputTokens": 0
            }
        });
        let md = parse_metadata(&meta).expect("应解析出元数据");
        assert_eq!(md.model.as_deref(), Some("OpenAI API (gpt-4o)"));
        assert_eq!(md.provider.as_deref(), Some("openai"));
        assert_eq!(md.first_token_latency_ms, Some(10319));
        assert_eq!(md.finish_reason.as_deref(), Some("stop"));
        let u = md.usage.expect("应有 usage");
        assert_eq!(u.input_tokens, Some(15));
        assert_eq!(u.output_tokens, Some(415));
        assert_eq!(u.total_tokens, Some(430));
    }

    #[test]
    fn parse_metadata_blank_yields_none() {
        assert!(parse_metadata(&serde_json::json!({})).is_none());
        assert!(parse_metadata(&serde_json::Value::Null).is_none());
        // 仅空串 / 空 usage 也视为无元数据。
        assert!(parse_metadata(&serde_json::json!({ "model": "  ", "usage": {} })).is_none());
    }

    #[test]
    fn sessions_json_maps_metadata_and_backfills_model() {
        let value = serde_json::json!([
            {
                "id": "s3",
                "name": "带统计对话",
                "messages": [
                    { "role": "user", "contentParts": [{ "type": "text", "text": "问" }] },
                    { "role": "assistant", "contentParts": [{ "type": "text", "text": "答" }],
                      "meta": {
                          "model": "OpenAI API (gemini-3-flash-preview)",
                          "provider": "openai",
                          "firstTokenLatency": 5949,
                          "finishReason": "stop",
                          "usage": { "totalTokens": 1792, "reasoningTokens": 595 }
                      }
                    }
                ]
            }
        ]);
        let convos = sessions_json_to_conversations(&value);
        assert_eq!(convos.len(), 1);
        // 会话级 model 由首条带模型的消息回填。
        assert_eq!(
            convos[0].model.as_deref(),
            Some("OpenAI API (gemini-3-flash-preview)")
        );
        // user 消息无元数据。
        assert!(convos[0].messages[0].metadata.is_none());
        // assistant 消息携带模型 / 延迟 / 用量。
        let md = convos[0].messages[1].metadata.as_ref().expect("assistant 应有元数据");
        assert_eq!(md.first_token_latency_ms, Some(5949));
        assert_eq!(md.usage.as_ref().unwrap().total_tokens, Some(1792));
        assert_eq!(md.usage.as_ref().unwrap().reasoning_tokens, Some(595));
    }

    #[test]
    fn metadata_json_roundtrips_through_parse() {
        // metadata_to_json（导入写回）与 parse_metadata（读取解析）应对称。
        let md = MessageMetadata {
            model: Some("OpenAI API (gpt-4o)".into()),
            provider: Some("openai".into()),
            usage: Some(TokenUsage {
                input_tokens: Some(15),
                output_tokens: Some(415),
                total_tokens: Some(430),
                reasoning_tokens: None,
                cached_input_tokens: None,
            }),
            first_token_latency_ms: Some(10319),
            finish_reason: Some("stop".into()),
        };
        let json = metadata_to_json(&md);
        // 空字段不应出现。
        assert!(json.get("usage").unwrap().get("reasoningTokens").is_none());
        let back = parse_metadata(&json).expect("应能解析回来");
        assert_eq!(back, md);
    }

    #[test]
    fn read_script_carries_metadata() {
        // 读取脚本必须把 model / 用量 / 首字延迟归一到每条消息的 meta。
        assert!(READ_SCRIPT.contains("firstTokenLatency"));
        assert!(READ_SCRIPT.contains("aiProvider"));
        assert!(READ_SCRIPT.contains("usage"));
        assert!(READ_SCRIPT.contains("meta"));
    }

    #[test]
    fn sessions_json_preserves_resolved_images() {
        // 读取脚本解析出的 image(url) 应映射为多模态消息，图片得以保留。
        let value = serde_json::json!([
            {
                "id": "s2",
                "name": "带图对话",
                "messages": [
                    { "role": "user", "contentParts": [
                        { "type": "text", "text": "这是啥" },
                        { "type": "image", "url": "data:image/png;base64,CCCC" }
                    ]}
                ]
            }
        ]);
        let convos = sessions_json_to_conversations(&value);
        assert_eq!(convos.len(), 1);
        let msg = &convos[0].messages[0];
        assert!(msg.content.has_image());
        assert_eq!(msg.text(), "这是啥[图片]"); // 文本视图仍以占位符表示图片
    }

    #[test]
    fn read_script_scopes_to_session_prefix() {
        assert!(READ_SCRIPT.contains("chatboxstore"));
        assert!(READ_SCRIPT.contains("session:"));
        assert!(READ_SCRIPT.contains("contentParts"));
        // 必须展开历史话题，而非只读当前 messages。
        assert!(READ_SCRIPT.contains("s.threads"));
        assert!(READ_SCRIPT.contains("threadName"));
        // 必须经 getStoreBlob 把图片 storageKey 解析成真正的字节。
        assert!(READ_SCRIPT.contains("getStoreBlob"));
        assert!(READ_SCRIPT.contains("storageKey"));
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
            "messages": [{ "role": "user", "parts": [
                { "type": "text", "text": "你好" },
                { "type": "image", "url": "data:image/png;base64,DDDD" }
            ]}]
        });
        let script = build_import_script("abc-123", &payload);
        assert!(script.contains("chatboxstore"));
        assert!(script.contains("keyvaluepairs"));
        assert!(script.contains("chat-sessions-list"));
        assert!(script.contains("\"abc-123\""));
        assert!(script.contains("你好"));
        assert!(script.contains("data:image/png;base64,DDDD"));
        // session 值以 JSON 字符串写入（localforage 兼容）。
        assert!(script.contains("JSON.stringify(session)"));
        // 图片块经 setStoreBlob 写入 ChatBox 的 blob 存储，引用 storageKey。
        assert!(script.contains("setStoreBlob"));
        assert!(script.contains("storageKey"));
        assert!(script.contains("picture:"));
        // 元数据写回：脚本含 applyMeta，还原 model/aiProvider/tokensUsed 等字段。
        assert!(script.contains("applyMeta"));
        assert!(script.contains("aiProvider"));
        assert!(script.contains("tokensUsed"));
    }

    #[test]
    fn import_payload_includes_metadata_for_assistant() {
        // 带元数据的 assistant 消息，导入 payload 应附 meta（写回 ChatBox 统计字段）。
        let mut convo = Conversation::new();
        convo.messages.push(ChatMessage::user("问"));
        convo.messages.push(ChatMessage {
            role: Role::Assistant,
            content: MessageContent::Text("答".into()),
            name: None,
            metadata: Some(MessageMetadata {
                model: Some("OpenAI API (gpt-4o)".into()),
                provider: Some("openai".into()),
                usage: Some(TokenUsage {
                    total_tokens: Some(430),
                    ..Default::default()
                }),
                first_token_latency_ms: Some(10319),
                finish_reason: Some("stop".into()),
            }),
        });
        // 复刻 import_to_chatbox 的 payload 构造逻辑。
        let messages: Vec<Value> = convo
            .messages
            .iter()
            .map(|m| {
                let mut obj =
                    serde_json::json!({ "role": role_str(m.role), "parts": message_to_parts(m) });
                if let Some(meta) = m.metadata.as_ref().filter(|md| !md.is_empty()) {
                    obj["meta"] = metadata_to_json(meta);
                }
                obj
            })
            .collect();
        // user 无 meta，assistant 有 meta。
        assert!(messages[0].get("meta").is_none());
        let meta = messages[1].get("meta").expect("assistant 应带 meta");
        assert_eq!(meta["model"], "OpenAI API (gpt-4o)");
        assert_eq!(meta["firstTokenLatency"], 10319);
        assert_eq!(meta["usage"]["totalTokens"], 430);
    }
}
