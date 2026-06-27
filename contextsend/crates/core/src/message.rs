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

/// 一条消息的 token 用量明细，对齐各家（OpenAI / Anthropic）通用口径。
///
/// 字段全部可选：来源应用提供多少就填多少（如 ChatBox 给出 input/output/total
/// 及 reasoning/cached 细分）。空字段不序列化，保持载荷紧凑。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    /// 输入（提示）token 数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// 输出（补全）token 数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// 总 token 数（通常 = input + output）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    /// 其中思维链 / 推理消耗的 token 数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    /// 命中缓存而无需重新计费的输入 token 数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u64>,
}

impl TokenUsage {
    /// 是否完全为空（所有字段均为 `None`），用于决定是否附带 usage。
    pub fn is_empty(&self) -> bool {
        self.input_tokens.is_none()
            && self.output_tokens.is_none()
            && self.total_tokens.is_none()
            && self.reasoning_tokens.is_none()
            && self.cached_input_tokens.is_none()
    }
}

/// 一条消息的生成元数据：模型、token 用量、首字延迟等。
///
/// 通常仅 assistant 消息携带，由支持的适配器（如 ChatBox）在读取时填充，
/// 随对话一起在设备间传输并可写回目标应用。**非 OpenAI 标准 message 字段**，
/// 故全部可选 + 空则不序列化：对纯文本 / OpenAI 导入的会话完全无影响，
/// 粘贴到 OpenAI 兼容端点时这些额外字段会被忽略。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageMetadata {
    /// 生成该消息所用的模型标识（来源应用的展示名，如 `OpenAI API (gpt-4o)`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// AI 提供方标识（如 `openai` / `anthropic`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// token 用量明细（空则不附带）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// 首 token 延迟（毫秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_token_latency_ms: Option<u64>,
    /// 结束原因（如 `stop` / `length` / `tool_calls`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

impl MessageMetadata {
    /// 是否完全为空（无任何元数据），用于决定是否附带到消息上。
    pub fn is_empty(&self) -> bool {
        self.model.is_none()
            && self.provider.is_none()
            && self.first_token_latency_ms.is_none()
            && self.finish_reason.is_none()
            && self.usage.as_ref().is_none_or(TokenUsage::is_empty)
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
    /// 可选的生成元数据（模型 / token 用量 / 首字延迟等）。
    /// 由支持的适配器读取时填充；纯文本 / OpenAI 导入的会话为 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

impl ChatMessage {
    /// 以指定角色与纯文本内容构造一条消息。
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: MessageContent::Text(content.into()),
            name: None,
            metadata: None,
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
            metadata: None,
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
        assert!(msg.metadata.is_none());
    }

    #[test]
    fn metadata_is_skipped_when_absent_and_roundtrips_when_present() {
        // 无元数据：序列化不应出现 metadata 字段（向后兼容、载荷紧凑）。
        let plain = ChatMessage::user("你好");
        let json = serde_json::to_string(&plain).unwrap();
        assert!(!json.contains("metadata"));

        // 有元数据：camelCase 字段往返一致。
        let msg = ChatMessage {
            role: Role::Assistant,
            content: MessageContent::Text("答复".into()),
            name: None,
            metadata: Some(MessageMetadata {
                model: Some("OpenAI API (gpt-4o)".into()),
                provider: Some("openai".into()),
                usage: Some(TokenUsage {
                    input_tokens: Some(15),
                    output_tokens: Some(415),
                    total_tokens: Some(430),
                    reasoning_tokens: Some(0),
                    cached_input_tokens: Some(0),
                }),
                first_token_latency_ms: Some(10319),
                finish_reason: Some("stop".into()),
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"firstTokenLatencyMs\":10319"));
        assert!(json.contains("\"totalTokens\":430"));
        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn metadata_is_empty_detects_blank() {
        assert!(MessageMetadata::default().is_empty());
        assert!(TokenUsage::default().is_empty());
        let with_model = MessageMetadata {
            model: Some("x".into()),
            ..Default::default()
        };
        assert!(!with_model.is_empty());
    }
}
