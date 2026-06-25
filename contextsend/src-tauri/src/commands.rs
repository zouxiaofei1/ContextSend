//! Tauri IPC command 层。
//!
//! 把分层 crate 的能力暴露给前端：应用信息、本机身份、导入/导出，
//! 以及网络层（设备列表、配对、推送、接收）。

use std::sync::Mutex;

use cs_core::openai::{export_openai_json, import_openai_json};
use cs_core::{ChatMessage, Conversation};
use cs_network::{Device, DeviceIdentity, NetworkService};
use serde::Serialize;
use tauri::State;

/// 应用级共享状态（注入到 Tauri `State`）。
pub struct AppState {
    /// 网络服务句柄（后台异步启动，就绪后填充）。
    pub service: tokio::sync::OnceCell<NetworkService>,
    /// 本机身份持久化文件路径。
    pub identity_path: std::path::PathBuf,
    /// 本机身份（可改名）。
    pub identity: Mutex<DeviceIdentity>,
    /// 关闭窗口时是否最小化到托盘（默认 true）。
    pub minimize_to_tray: Mutex<bool>,
    /// 是否开机自启（由托盘菜单管理）。
    pub auto_start: Mutex<bool>,
    /// 当前在线设备快照 `(id, name)`，供托盘菜单使用。
    pub online_devices: Mutex<Vec<(String, String)>>,
}

impl AppState {
    /// 取已就绪的网络服务克隆（句柄内部 `Arc`，克隆廉价）；未就绪时返回友好错误。
    fn service(&self) -> Result<NetworkService, String> {
        self.service
            .get()
            .cloned()
            .ok_or_else(|| "网络服务尚未就绪，请稍候".to_string())
    }
}

/// 返回给前端的应用基础信息。
#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub adapters: Vec<String>,
}

/// 获取应用信息。
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        adapters: cs_adapters::builtin_adapter_names()
            .into_iter()
            .map(String::from)
            .collect(),
    }
}

/// 本机身份视图。
#[derive(Debug, Serialize)]
pub struct SelfIdentity {
    pub uuid: String,
    pub name: String,
}

/// 获取本机身份（UUID + 显示名）。
#[tauri::command]
pub fn get_self_identity(state: State<'_, AppState>) -> SelfIdentity {
    let id = state.identity.lock().unwrap();
    SelfIdentity {
        uuid: id.uuid.clone(),
        name: id.name.clone(),
    }
}

/// 给本机改名并持久化。
#[tauri::command]
pub fn rename_self(state: State<'_, AppState>, new_name: String) -> Result<(), String> {
    let mut id = state.identity.lock().unwrap();
    id.rename(new_name, &state.identity_path)
        .map_err(|e| e.to_string())
}

/// 当前设备列表快照。
#[tauri::command]
pub fn list_devices(state: State<'_, AppState>) -> Vec<Device> {
    state
        .service
        .get()
        .map(|s| s.list_devices())
        .unwrap_or_default()
}

/// 配对发起结果。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingStarted {
    pub pairing_id: u64,
    pub pin: String,
}

/// 主动向目标设备发起配对，返回 6 位配对码供用户比对。
#[tauri::command]
pub async fn connect_pair(
    state: State<'_, AppState>,
    target_uuid: String,
) -> Result<PairingStarted, String> {
    log::info!("发起配对: target_uuid={target_uuid}");
    let started = state
        .service()?
        .connect_pair(&target_uuid)
        .await
        .map_err(|e| {
            log::warn!("发起配对失败: target_uuid={target_uuid} err={e}");
            e.to_string()
        })?;
    log::info!(
        "配对已建立(待用户确认配对码): pairing_id={} target_uuid={target_uuid}",
        started.0
    );
    Ok(PairingStarted {
        pairing_id: started.0,
        pin: started.1,
    })
}

/// 用户确认配对码一致后，推送一段对话。
#[tauri::command]
pub async fn push_conversation(
    state: State<'_, AppState>,
    pairing_id: u64,
    conversation: Conversation,
) -> Result<(), String> {
    log::info!(
        "推送对话: pairing_id={pairing_id} messages={} title={:?}",
        conversation.messages.len(),
        conversation.title
    );
    state
        .service()?
        .push(pairing_id, &conversation)
        .await
        .map(|_| log::info!("推送完成: pairing_id={pairing_id}"))
        .map_err(|e| {
            log::warn!("推送失败: pairing_id={pairing_id} err={e}");
            e.to_string()
        })
}

/// 用户确认入站配对码一致后，接收对端推送的对话（结果经 `net-event` 事件抛出）。
#[tauri::command]
pub async fn accept_incoming(state: State<'_, AppState>, pairing_id: u64) -> Result<(), String> {
    log::info!("接受入站对话: pairing_id={pairing_id}");
    state
        .service()?
        .accept_incoming(pairing_id)
        .await
        .map(|_| log::info!("入站对话接收完成: pairing_id={pairing_id}"))
        .map_err(|e| {
            log::warn!("接收入站对话失败: pairing_id={pairing_id} err={e}");
            e.to_string()
        })
}

/// 拒绝/取消一个待确认配对。
#[tauri::command]
pub fn reject_pairing(state: State<'_, AppState>, pairing_id: u64) {
    log::info!("拒绝/取消配对: pairing_id={pairing_id}");
    if let Some(s) = state.service.get() {
        s.reject(pairing_id);
    }
}

/// 解析一段 OpenAI Compatible JSON 文本为内部对话结构。
#[tauri::command]
pub fn import_openai(json: String) -> Result<Conversation, String> {
    import_openai_json(&json).map_err(|e| e.to_string())
}

/// 将一段对话导出为 OpenAI Compatible JSON 文本。
#[tauri::command]
pub fn export_openai(conversation: Conversation) -> Result<String, String> {
    export_openai_json(&conversation).map_err(|e| e.to_string())
}

/// 导入结果：新建会话的 id 与目标应用名（供前端提示用户切回该应用查看）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub app: String,
    pub thread_id: String,
}

/// 把一段对话导入到本机指定的 Chat AI 应用（写入其存储，使其出现新会话标签页）。
///
/// - **Jan**：写新 thread 目录（需切回 Jan 窗口或重启刷新）。
/// - **ChatBox**：经 CDP 注入渲染进程写 IndexedDB 并自动刷新侧栏；需 ChatBox 已带
///   `--remote-debugging-port=9222` 启动，否则返回提示错误。
#[tauri::command]
pub async fn import_to_app(
    app: String,
    conversation: Conversation,
) -> Result<ImportResult, String> {
    log::info!(
        "导入到本地应用: app={app} messages={}",
        conversation.messages.len()
    );
    let thread_id = cs_adapters::import_conversation_to(&app, &conversation)
        .await
        .map_err(|e| {
            log::warn!("导入失败: app={app} err={e}");
            e.to_string()
        })?;
    log::info!("导入完成: app={app} thread_id={thread_id}");
    Ok(ImportResult { app, thread_id })
}

/// 片段匹配结果：是否命中本地某会话，命中则附来源应用与得分。
///
/// `conversation` 始终有值：命中时是匹配到的完整会话，未命中时是把片段本身
/// 包成单条用户消息的占位会话，便于前端统一加入存储库。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchOutcome {
    pub matched: bool,
    pub app: Option<String>,
    pub score: f32,
    pub conversation: Conversation,
}

/// 把一段复制 / 拖入的上下文片段，匹配回本地应用里的完整会话（导出方向）。
///
/// 片段过短会返回错误；命中则返回整条会话，未命中则把片段包成占位会话返回。
#[tauri::command]
pub async fn match_context(snippet: String) -> Result<MatchOutcome, String> {
    log::info!("片段匹配请求: snippet_chars={}", snippet.chars().count());
    match cs_adapters::match_snippet(&snippet).await.map_err(|e| {
        log::warn!("片段匹配出错: err={e}");
        e.to_string()
    })? {
        Some(m) => {
            log::info!("片段命中: app={} score={:.3}", m.app, m.score);
            Ok(MatchOutcome {
                matched: true,
                app: Some(m.app),
                score: m.score,
                conversation: m.conversation,
            })
        }
        None => {
            log::info!("片段未命中，已包成占位会话返回");
            // 未匹配到：把片段本身作为一段独立会话，仍纳入存储库。
            let mut conversation = Conversation::new();
            conversation.messages.push(ChatMessage::user(snippet));
            Ok(MatchOutcome {
                matched: false,
                app: None,
                score: 0.0,
                conversation,
            })
        }
    }
}

/// 设置关闭窗口时是否最小化到托盘。
#[tauri::command]
pub fn set_minimize_to_tray(state: State<'_, AppState>, enabled: bool) {
    *state.minimize_to_tray.lock().unwrap() = enabled;
}
