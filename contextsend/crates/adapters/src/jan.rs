//! Jan AI 适配器：读写 Jan 的文件系统对话存储。
//!
//! Jan（桌面端）把每个会话（thread）存为一个目录：
//!
//! ```text
//! <data_folder>/threads/<thread_id>/
//!     ├── thread.json      # 会话元数据（title / assistants / created / updated）
//!     └── messages.jsonl   # 每行一条消息（OpenAI 风格 content 块）
//! ```
//!
//! `<data_folder>` 默认是 `%APPDATA%/Jan/data`（Windows），也可被
//! `%APPDATA%/Jan/settings.json` 的 `data_folder` 字段覆盖（与 Jan 自身一致）。
//!
//! 导入：写入一个新的 thread 目录 —— Jan 不监听磁盘，但**窗口重新获得焦点时会
//! 重新读取 threads 目录**（`fetchThreads`），因此切回 Jan 窗口即可看到新标签页，
//! 无需重启。Jan 不会删除它不认识的 thread，故运行时写入也是安全的。

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cs_core::{ChatMessage, ContentPart, Conversation, MessageContent, Role};

use crate::{Adapter, AdapterError};

/// Jan AI 适配器。
pub struct JanAdapter;

impl JanAdapter {
    /// Jan 的应用配置目录：`%APPDATA%/Jan`（Windows）/ `~/.local/share/Jan`（Linux）等。
    fn app_data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("Jan"))
    }

    /// Jan 的数据目录（`data_folder`）。优先读取 `settings.json` 的 `data_folder`，
    /// 回退到默认 `<app_data>/data`，与 Jan 的 `resolve_jan_data_folder` 行为一致。
    fn data_folder() -> Option<PathBuf> {
        let app_data = Self::app_data_dir()?;
        let settings = app_data.join("settings.json");
        if let Ok(content) = fs::read_to_string(&settings) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(folder) = v.get("data_folder").and_then(|f| f.as_str()) {
                    if !folder.is_empty() {
                        return Some(PathBuf::from(folder));
                    }
                }
            }
        }
        Some(app_data.join("data"))
    }

    /// `<data_folder>/threads`。
    fn threads_dir() -> Option<PathBuf> {
        Some(Self::data_folder()?.join("threads"))
    }
}

/// 当前 Unix 时间（秒）。Jan 的时间戳是秒级。
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 把 cs-core 的 [`Role`] 映射为 Jan/OpenAI 的角色字符串。
fn role_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

/// 把一条 cs-core 消息的内容映射为 Jan 的 `content[]` 数组。
fn content_blocks(content: &MessageContent) -> serde_json::Value {
    let mut blocks = Vec::new();
    match content {
        MessageContent::Text(s) => {
            blocks.push(text_block(s));
        }
        MessageContent::Parts(parts) => {
            for part in parts {
                match part {
                    ContentPart::Text { text } => blocks.push(text_block(text)),
                    ContentPart::ImageUrl { image_url } => {
                        blocks.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": image_url.url }
                        }));
                    }
                }
            }
        }
    }
    // 空内容也要保证是合法的 content 数组，避免 Jan 渲染异常。
    if blocks.is_empty() {
        blocks.push(text_block(""));
    }
    serde_json::Value::Array(blocks)
}

/// Jan 的文本内容块：`{ "type": "text", "text": { "value": ..., "annotations": [] } }`。
fn text_block(value: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "text",
        "text": { "value": value, "annotations": [] }
    })
}

/// 从 Jan 的 `content[]` 还原纯文本（拼接所有 text 块，图像以占位符表示）。
fn content_to_text(content: &serde_json::Value) -> String {
    let Some(arr) = content.as_array() else {
        return String::new();
    };
    let mut out = String::new();
    for block in arr {
        match block.get("type").and_then(|t| t.as_str()) {
            Some("text") | Some("reasoning") => {
                if let Some(v) = block.pointer("/text/value").and_then(|v| v.as_str()) {
                    out.push_str(v);
                }
            }
            Some("image_url") => out.push_str("[图片]"),
            _ => {}
        }
    }
    out
}

impl Adapter for JanAdapter {
    fn app_name(&self) -> &'static str {
        "Jan"
    }

    fn data_dir(&self) -> Option<PathBuf> {
        // 仅当 threads 目录确实存在时才认为 Jan 已安装并使用过。
        let dir = Self::threads_dir()?;
        dir.exists().then_some(dir)
    }

    fn list_conversations(&self) -> Result<Vec<Conversation>, AdapterError> {
        let threads_dir = Self::threads_dir().ok_or(AdapterError::AppNotFound)?;
        if !threads_dir.exists() {
            log::debug!("Jan threads 目录不存在，返回空: {}", threads_dir.display());
            return Ok(Vec::new());
        }
        log::debug!("Jan 读取会话: dir={}", threads_dir.display());

        let mut convos = Vec::new();
        for entry in fs::read_dir(&threads_dir)? {
            let entry = entry?;
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }

            let meta_path = dir.join("thread.json");
            let Ok(meta_raw) = fs::read_to_string(&meta_path) else {
                continue; // 没有 thread.json 的目录跳过
            };
            let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_raw) else {
                continue; // 损坏的元数据跳过，不让单条坏数据拖垮整体
            };

            let title = meta
                .get("title")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());

            let mut convo = Conversation::new();
            convo.title = title;

            let messages_path = dir.join("messages.jsonl");
            if let Ok(jsonl) = fs::read_to_string(&messages_path) {
                for line in jsonl.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
                        continue;
                    };
                    let role = match msg.get("role").and_then(|r| r.as_str()) {
                        Some("system") => Role::System,
                        Some("assistant") => Role::Assistant,
                        Some("tool") => Role::Tool,
                        _ => Role::User,
                    };
                    let text = msg.get("content").map(content_to_text).unwrap_or_default();
                    convo.messages.push(ChatMessage::new(role, text));
                }
            }

            convos.push(convo);
        }

        log::debug!("Jan 读取完成: 共 {} 个会话", convos.len());
        Ok(convos)
    }

    fn import_conversation(&self, convo: &Conversation) -> Result<String, AdapterError> {
        let threads_dir = Self::threads_dir().ok_or(AdapterError::AppNotFound)?;

        let thread_id = uuid::Uuid::new_v4().to_string();
        let thread_dir = threads_dir.join(&thread_id);
        fs::create_dir_all(&thread_dir)?;

        let now = now_secs();
        let title = convo
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "来自 ContextSend".to_string());

        // 1) thread.json —— 会话元数据。assistants 留空，用户打开后自行选模型。
        let thread_meta = serde_json::json!({
            "id": thread_id,
            "object": "thread",
            "title": title,
            "assistants": [],
            "created": now,
            "updated": now,
            "metadata": { "source": "contextsend" }
        });
        let meta_str = serde_json::to_string_pretty(&thread_meta)?;
        fs::write(thread_dir.join("thread.json"), meta_str)?;

        // 2) messages.jsonl —— 每行一条消息。
        let messages_path = thread_dir.join("messages.jsonl");
        let mut file = fs::File::create(&messages_path)?;
        for msg in &convo.messages {
            let msg_json = serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "object": "thread.message",
                "thread_id": thread_id,
                "role": role_str(msg.role),
                "content": content_blocks(&msg.content),
                "status": "ready",
                "created_at": now,
                "completed_at": now,
            });
            // 紧凑写一行，行内不得含换行（jsonl 约定）。
            writeln!(file, "{}", serde_json::to_string(&msg_json)?)?;
        }

        log::info!(
            "Jan 写入会话: thread_id={thread_id} messages={}",
            convo.messages.len()
        );
        Ok(thread_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_maps_to_openai_strings() {
        assert_eq!(role_str(Role::User), "user");
        assert_eq!(role_str(Role::Assistant), "assistant");
        assert_eq!(role_str(Role::System), "system");
    }

    #[test]
    fn text_content_becomes_jan_block() {
        let blocks = content_blocks(&MessageContent::Text("你好".into()));
        assert_eq!(blocks[0]["type"], "text");
        assert_eq!(blocks[0]["text"]["value"], "你好");
        assert!(blocks[0]["text"]["annotations"].is_array());
    }

    #[test]
    fn empty_content_still_yields_a_block() {
        let blocks = content_blocks(&MessageContent::Parts(vec![]));
        assert_eq!(blocks.as_array().unwrap().len(), 1);
        assert_eq!(blocks[0]["text"]["value"], "");
    }

    #[test]
    fn jan_content_roundtrips_to_text() {
        let content = serde_json::json!([
            { "type": "text", "text": { "value": "看这张图", "annotations": [] } },
            { "type": "image_url", "image_url": { "url": "x" } },
        ]);
        assert_eq!(content_to_text(&content), "看这张图[图片]");
    }

    #[test]
    fn import_then_read_roundtrips_via_temp_dir() {
        // 用临时目录模拟 Jan 的 threads 目录，验证写入的文件能被自身读回。
        let tmp = std::env::temp_dir().join(format!("cs-jan-test-{}", uuid::Uuid::new_v4()));
        let thread_id = uuid::Uuid::new_v4().to_string();
        let thread_dir = tmp.join(&thread_id);
        fs::create_dir_all(&thread_dir).unwrap();

        let now = now_secs();
        let thread_meta = serde_json::json!({
            "id": thread_id, "object": "thread", "title": "测试会话",
            "assistants": [], "created": now, "updated": now,
        });
        fs::write(
            thread_dir.join("thread.json"),
            serde_json::to_string_pretty(&thread_meta).unwrap(),
        )
        .unwrap();

        let msg = serde_json::json!({
            "id": "m1", "object": "thread.message", "thread_id": thread_id,
            "role": "user",
            "content": [{ "type": "text", "text": { "value": "你好 Jan", "annotations": [] } }],
            "status": "ready", "created_at": now, "completed_at": now,
        });
        fs::write(
            thread_dir.join("messages.jsonl"),
            format!("{}\n", serde_json::to_string(&msg).unwrap()),
        )
        .unwrap();

        // 读回 thread.json 的 title 与首条消息文本。
        let meta: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(thread_dir.join("thread.json")).unwrap())
                .unwrap();
        assert_eq!(meta["title"], "测试会话");
        let line = fs::read_to_string(thread_dir.join("messages.jsonl")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(content_to_text(&parsed["content"]), "你好 Jan");

        let _ = fs::remove_dir_all(&tmp);
    }
}
