//! ContextSend 核心引擎（cs-core）。
//!
//! 本 crate 定义跨应用通用的对话数据格式，对齐 **OpenAI Chat Completion** 结构，
//! 作为所有 Adapter 提取上下文后归一化的目标格式，也是 Network layer 传输的载荷格式。
//!
//! - [`message`]：消息与多模态内容（[`ChatMessage`] / [`MessageContent`] / [`ContentPart`]）。
//! - [`openai`]：OpenAI Compatible JSON 的导入 / 导出。

mod message;
pub mod openai;

pub use message::{
    ChatMessage, ContentPart, ImageUrl, MessageContent, MessageMetadata, Role, TokenUsage,
};

use serde::{Deserialize, Serialize};

/// 一段完整对话上下文，是 ContextSend 在应用间传输的基本单位。
///
/// 字段命名对齐 OpenAI Chat Completion 请求体，便于「拖入 JSON 文件」直接复用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    /// 对话标题（来源应用提供，可能为空）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 生成该对话所用的模型标识（可能为空）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 有序的消息列表。
    pub messages: Vec<ChatMessage>,
}

impl Conversation {
    /// 创建一段空对话。
    pub fn new() -> Self {
        Self {
            title: None,
            model: None,
            messages: Vec::new(),
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

/// cs-core 的统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("序列化/反序列化失败: {0}")]
    Serde(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_roundtrips_through_openai_json() {
        let mut convo = Conversation::new();
        convo.model = Some("gpt-4o".to_string());
        convo.messages.push(ChatMessage::system("你是一个助手"));
        convo.messages.push(ChatMessage::user("你好"));

        let json = serde_json::to_string(&convo).expect("序列化应成功");
        // 角色应按 OpenAI 约定小写
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"role\":\"system\""));

        let back: Conversation = serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(convo, back);
    }
}
