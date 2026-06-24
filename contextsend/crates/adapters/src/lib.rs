//! ContextSend 适配器层（cs-adapters）。
//!
//! 每个 Adapter 负责定位某个本地 Chat AI 应用（如 ChatBox、Jan）的数据目录，
//! 读取其对话存储，并归一化为 [`cs_core::Conversation`]。
//!
//! Phase 0 仅定义统一的 [`Adapter`] trait 与首批适配器的占位实现；
//! 真实的数据目录定位与解析逻辑在 Phase 1 接入。

use cs_core::Conversation;

/// 适配器错误类型。
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("尚未实现")]
    NotImplemented,
    #[error("未找到应用数据目录")]
    AppNotFound,
    #[error("核心错误: {0}")]
    Core(#[from] cs_core::CoreError),
}

/// 所有适配器需实现的统一接口。
///
/// 该 trait 故意保持最小：远期会扩展为「列出对话 / 读取单个对话 / 写回（迁移）」等能力，
/// 并通过 `WASM 插件系统` 支持第三方适配器。
pub trait Adapter {
    /// 适配器对应的应用名（用于设备/来源展示）。
    fn app_name(&self) -> &'static str;

    /// 列出可提取的对话。Phase 0 返回未实现错误。
    fn list_conversations(&self) -> Result<Vec<Conversation>, AdapterError>;
}

/// ChatBox 适配器（首批，占位）。
pub struct ChatBoxAdapter;

impl Adapter for ChatBoxAdapter {
    fn app_name(&self) -> &'static str {
        "ChatBox"
    }

    fn list_conversations(&self) -> Result<Vec<Conversation>, AdapterError> {
        Err(AdapterError::NotImplemented)
    }
}

/// Jan AI 适配器（首批，占位）。
pub struct JanAdapter;

impl Adapter for JanAdapter {
    fn app_name(&self) -> &'static str {
        "Jan"
    }

    fn list_conversations(&self) -> Result<Vec<Conversation>, AdapterError> {
        Err(AdapterError::NotImplemented)
    }
}

/// 返回当前内置的所有适配器名称，供 UI / 权限作用域参考。
pub fn builtin_adapter_names() -> Vec<&'static str> {
    vec![ChatBoxAdapter.app_name(), JanAdapter.app_name()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_adapters_are_registered() {
        let names = builtin_adapter_names();
        assert!(names.contains(&"ChatBox"));
        assert!(names.contains(&"Jan"));
    }
}
