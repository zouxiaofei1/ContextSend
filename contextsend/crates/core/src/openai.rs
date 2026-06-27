//! OpenAI Compatible JSON 的导入与导出。
//!
//! 导入兼容三种常见形态：
//! 1. 完整请求体 `{"model": "...", "messages": [...]}`；
//! 2. 裸消息数组 `[ {"role":...,"content":...}, ... ]`；
//! 3. 带额外字段的请求体（未知字段忽略）。
//!
//! 导出固定为标准请求体形态 `{"model"?, "messages": [...]}`，可直接粘贴到
//! ChatGPT Web / 任意 OpenAI 兼容输入框。

use serde::Deserialize;

use crate::{ChatMessage, Conversation, CoreError};

/// 完整请求体形态（导入用），仅取我们关心的字段，其余忽略。
#[derive(Debug, Deserialize)]
struct OpenAiRequest {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    title: Option<String>,
    messages: Vec<ChatMessage>,
}

/// 导入的中间表示：要么是完整请求体，要么是裸消息数组。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenAiImport {
    Request(OpenAiRequest),
    Messages(Vec<ChatMessage>),
}

/// 从 OpenAI Compatible JSON 文本解析出一段 [`Conversation`]。
///
/// 同时接受「完整请求体」与「裸消息数组」两种输入。
pub fn import_openai_json(json: &str) -> Result<Conversation, CoreError> {
    let imported: OpenAiImport = serde_json::from_str(json)?;
    let convo = match imported {
        OpenAiImport::Request(req) => Conversation {
            title: req.title,
            model: req.model,
            messages: req.messages,
        },
        OpenAiImport::Messages(messages) => Conversation {
            title: None,
            model: None,
            messages,
        },
    };
    Ok(convo)
}

/// 将一段 [`Conversation`] 导出为标准 OpenAI 请求体 JSON（带缩进，便于阅读/粘贴）。
///
/// 输出始终包含 `messages`；`model`/`title` 仅在存在时输出。
pub fn export_openai_json(convo: &Conversation) -> Result<String, CoreError> {
    // 直接复用 Conversation 的 Serialize：字段名与跳过空值规则已对齐 OpenAI。
    Ok(serde_json::to_string_pretty(convo)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentPart, ImageUrl, MessageContent, Role};

    #[test]
    fn imports_full_request_body() {
        let json = r#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "你是助手"},
                {"role": "user", "content": "你好"}
            ]
        }"#;
        let convo = import_openai_json(json).unwrap();
        assert_eq!(convo.model.as_deref(), Some("gpt-4o"));
        assert_eq!(convo.messages.len(), 2);
        assert_eq!(convo.messages[1].text(), "你好");
    }

    #[test]
    fn imports_bare_messages_array() {
        let json = r#"[{"role":"user","content":"hi"}]"#;
        let convo = import_openai_json(json).unwrap();
        assert!(convo.model.is_none());
        assert_eq!(convo.messages.len(), 1);
        assert_eq!(convo.messages[0].role, Role::User);
    }

    #[test]
    fn ignores_unknown_fields() {
        let json = r#"{
            "model": "x",
            "temperature": 0.7,
            "stream": true,
            "messages": [{"role":"user","content":"a"}]
        }"#;
        let convo = import_openai_json(json).unwrap();
        assert_eq!(convo.messages.len(), 1);
    }

    #[test]
    fn export_then_import_roundtrips() {
        let convo = Conversation {
            title: Some("演示".into()),
            model: Some("gpt-4o".into()),
            messages: vec![
                ChatMessage::system("系统提示"),
                ChatMessage::user("问题"),
                ChatMessage::assistant("回答"),
            ],
        };
        let json = export_openai_json(&convo).unwrap();
        let back = import_openai_json(&json).unwrap();
        assert_eq!(convo, back);
    }

    #[test]
    fn multimodal_message_roundtrips_through_export() {
        let convo = Conversation {
            title: None,
            model: None,
            messages: vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Parts(vec![
                    ContentPart::Text {
                        text: "图里是啥".into(),
                    },
                    ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "data:image/png;base64,AAAA".into(),
                            detail: None,
                        },
                    },
                ]),
                name: None,
                metadata: None,
            }],
        };
        let json = export_openai_json(&convo).unwrap();
        let back = import_openai_json(&json).unwrap();
        assert_eq!(convo, back);
    }
}
