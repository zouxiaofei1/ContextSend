//! ContextSend 适配器层（cs-adapters）。
//!
//! 每个 Adapter 负责定位某个本地 Chat AI 应用（如 ChatBox、Jan）的数据目录，
//! 在它与 [`cs_core::Conversation`] 之间双向转换：
//!
//! - **读取 / 导出**：[`Adapter::list_conversations`] 把应用里的对话归一化出来。
//! - **写入 / 导入**：[`Adapter::import_conversation`] 把一段对话写进应用存储，
//!   使其作为新会话出现（Jan：写新 thread 目录，窗口聚焦即刷新）。
//!
//! 首批适配器：
//! - **Jan**（[`jan`]）：纯文件系统（JSON + JSONL），读写均已实现。
//! - **ChatBox**（[`chatbox`]）：桌面端把会话存进 Chromium IndexedDB。读写均经
//!   Chrome DevTools Protocol 注入渲染进程，用页面自身 localforage 存储格式
//!   读 / 写（需带 `--remote-debugging-port=9222` 启动 ChatBox）：导入见
//!   [`import_to_chatbox`]，读取见 [`list_chatbox_conversations`]。

mod chatbox;
mod jan;
mod matching;

pub use chatbox::{import_to_chatbox, list_chatbox_conversations};
pub use jan::JanAdapter;
pub use matching::{match_snippet, ConversationMatch, MIN_SNIPPET_CHARS};

use std::path::PathBuf;

use cs_core::Conversation;

/// 适配器错误类型。
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("尚未实现")]
    NotImplemented,
    #[error("未找到应用数据目录")]
    AppNotFound,
    #[error("无此适配器: {0}")]
    UnknownAdapter(String),
    #[error("内容太短，至少需要 {0} 个字符才能可靠匹配")]
    SnippetTooShort(usize),
    #[error("文件读写失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化/反序列化失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("核心错误: {0}")]
    Core(#[from] cs_core::CoreError),
    #[error("ChatBox 导入失败: {0}")]
    ChatBox(String),
}

/// 所有适配器需实现的统一接口。
///
/// 该 trait 覆盖「导入（写入目标应用）」与「读取（导出 / 匹配）」两个方向，
/// 远期会扩展为「按 id 读取单个会话 / 片段匹配」等能力，并通过 WASM 插件支持第三方适配器。
pub trait Adapter {
    /// 适配器对应的应用名（用于设备 / 来源展示与查找）。
    fn app_name(&self) -> &'static str;

    /// 目标应用的数据目录；未安装 / 未使用过返回 `None`。
    fn data_dir(&self) -> Option<PathBuf>;

    /// 目标应用是否已安装（默认以 [`data_dir`](Adapter::data_dir) 是否存在判断）。
    fn is_installed(&self) -> bool {
        self.data_dir().is_some()
    }

    /// 列出可提取的对话（读取 / 导出方向）。
    fn list_conversations(&self) -> Result<Vec<Conversation>, AdapterError>;

    /// 把一段对话写入目标应用存储，返回新建会话的 id（导入方向）。
    ///
    /// 默认未实现（占位适配器用）。已实现的适配器（如 Jan）覆盖此方法。
    fn import_conversation(&self, _convo: &Conversation) -> Result<String, AdapterError> {
        Err(AdapterError::NotImplemented)
    }
}

/// ChatBox 适配器。
///
/// 桌面端 ChatBox 把会话存进 Chromium IndexedDB（localforage）。**读取**需解析
/// leveldb，暂未实现；**导入**不走同步 trait（CDP 是异步且需目标进程在运行），
/// 改由 [`import_to_chatbox`] 处理，[`import_conversation_to`] 会自动路由。
pub struct ChatBoxAdapter;

impl ChatBoxAdapter {
    /// ChatBox 的应用数据目录：`%APPDATA%/xyz.chatboxapp.app`（Windows）。
    /// 仅用于「是否安装」判断；导入本身不依赖它（走 CDP）。
    fn app_data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("xyz.chatboxapp.app"))
    }
}

impl Adapter for ChatBoxAdapter {
    fn app_name(&self) -> &'static str {
        "ChatBox"
    }

    fn data_dir(&self) -> Option<PathBuf> {
        let dir = Self::app_data_dir()?;
        dir.exists().then_some(dir)
    }

    fn list_conversations(&self) -> Result<Vec<Conversation>, AdapterError> {
        Err(AdapterError::NotImplemented)
    }

    // 注意：import_conversation（同步）保持默认 NotImplemented——ChatBox 导入是异步
    // CDP 路径，经 import_conversation_to / import_to_chatbox 调用，不走同步 trait。
}

/// 返回当前内置的所有适配器名称，供 UI / 权限作用域参考。
pub fn builtin_adapter_names() -> Vec<&'static str> {
    vec![JanAdapter.app_name(), ChatBoxAdapter.app_name()]
}

/// 按应用名取得对应适配器（大小写不敏感）。未知名字返回 [`AdapterError::UnknownAdapter`]。
pub fn adapter_by_name(name: &str) -> Result<Box<dyn Adapter>, AdapterError> {
    match name.to_ascii_lowercase().as_str() {
        "jan" => Ok(Box::new(JanAdapter)),
        "chatbox" => Ok(Box::new(ChatBoxAdapter)),
        other => Err(AdapterError::UnknownAdapter(other.to_string())),
    }
}

/// 把一段对话导入到指定应用，返回新建会话 id。供 IPC command 直接调用。
///
/// 路由：ChatBox 走异步 CDP 注入（[`import_to_chatbox`]），其余适配器走同步
/// [`Adapter::import_conversation`]（如 Jan 写文件）。
pub async fn import_conversation_to(
    app_name: &str,
    convo: &Conversation,
) -> Result<String, AdapterError> {
    if app_name.eq_ignore_ascii_case("chatbox") {
        return import_to_chatbox(convo).await;
    }
    adapter_by_name(app_name)?.import_conversation(convo)
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

    #[test]
    fn adapter_by_name_is_case_insensitive() {
        assert_eq!(adapter_by_name("JAN").unwrap().app_name(), "Jan");
        assert_eq!(adapter_by_name("chatbox").unwrap().app_name(), "ChatBox");
        assert!(adapter_by_name("unknown").is_err());
    }

    #[test]
    fn chatbox_sync_import_is_not_implemented() {
        // ChatBox 同步 trait 导入仍是 NotImplemented——真正的导入走异步 CDP 路径
        // （import_conversation_to / import_to_chatbox），此处仅确认未误实现同步版。
        let convo = Conversation::new();
        assert!(matches!(
            ChatBoxAdapter.import_conversation(&convo),
            Err(AdapterError::NotImplemented)
        ));
    }
}
