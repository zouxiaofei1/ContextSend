//! 对话消息与多模态内容。
//!
//! 结构对齐 OpenAI Chat Completion 的 `messages[]`：每条消息有 `role` 与 `content`，
//! 其中 `content` 既可以是纯字符串，也可以是「内容块数组」（文本 + 图像），后者用于多模态。

use serde::{Deserialize, Serialize};

/// 一条对话消息中的角色，对齐 OpenAI Chat Completion 的 `role` 字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 图像引用，对齐 OpenAI 的 `image_url` 对象。
///
/// `url` 既可以是 `http(s)://` 链接，也可以是 `data:image/...;base64,` 内联数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    /// 细节级别（`auto` / `low` / `high`），可选。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 多模态内容中的单个内容块，序列化为 `{"type":"text",...}` / `{"type":"image_url",...}`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// 文本块。
    Text { text: String },
    /// 图像块。
    ImageUrl {
        #[serde(rename = "image_url")]
        image_url: ImageUrl,
    },
}

/// 一条消息的内容：要么是纯文本，要么是多模态内容块数组。
///
/// serde 使用 `untagged`：JSON 中字符串解析为 [`MessageContent::Text`]，
/// 数组解析为 [`MessageContent::Parts`]，与 OpenAI 的两种 `content` 形态一致。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// 纯文本内容。
    Text(String),
    /// 多模态内容块（文本 / 图像混合）。
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// 提取纯文本视图：拼接所有文本块，忽略图像（图像以占位符表示）。
    ///
    /// 供迁移、预览、SAS 之外的「只关心文字」场景使用。
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        ContentPart::Text { text } => out.push_str(text),
                        ContentPart::ImageUrl { .. } => out.push_str("[图片]"),
                    }
                }
                out
            }
        }
    }

    /// 是否包含至少一个图像块。
    pub fn has_image(&self) -> bool {
        matches!(self, MessageContent::Parts(parts)
            if parts.iter().any(|p| matches!(p, ContentPart::ImageUrl { .. })))
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

/// 单条对话消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: MessageContent,
    /// 可选的消息名（OpenAI 中用于区分 tool / 多用户场景）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    /// 以指定角色与纯文本内容构造一条消息。
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: MessageContent::Text(content.into()),
            name: None,
        }
    }

    /// 便捷构造一条系统消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// 便捷构造一条用户消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// 便捷构造一条助手消息。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// 提取该消息的纯文本视图（见 [`MessageContent::as_text`]）。
    pub fn text(&self) -> String {
        self.content.as_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_content_serializes_as_string() {
        let msg = ChatMessage::user("你好");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["content"], serde_json::json!("你好"));
    }

    #[test]
    fn multimodal_content_roundtrips() {
        let msg = ChatMessage {
            role: Role::User,
            content: MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "看这张图".into(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/a.png".into(),
                        detail: Some("high".into()),
                    },
                },
            ]),
            name: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"type\":\"image_url\""));

        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
        assert!(back.content.has_image());
        assert_eq!(back.text(), "看这张图[图片]");
    }

    #[test]
    fn string_content_deserializes_into_text_variant() {
        let msg: ChatMessage =
            serde_json::from_str(r#"{"role":"assistant","content":"嗯"}"#).unwrap();
        assert_eq!(msg.content, MessageContent::Text("嗯".into()));
        assert_eq!(msg.text(), "嗯");
    }
}
