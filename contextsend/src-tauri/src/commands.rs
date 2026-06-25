//! Tauri IPC command 层。
//!
//! 把分层 crate 的能力暴露给前端：应用信息、本机身份、导入/导出，
//! 以及网络层（设备列表、配对、推送、接收）。

use std::sync::Mutex;

use cs_core::openai::{export_openai_json, import_openai_json};
use cs_core::Conversation;
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
    let (pairing_id, pin) = state
        .service()?
        .connect_pair(&target_uuid)
        .await
        .map_err(|e| e.to_string())?;
    Ok(PairingStarted { pairing_id, pin })
}

/// 用户确认配对码一致后，推送一段对话。
#[tauri::command]
pub async fn push_conversation(
    state: State<'_, AppState>,
    pairing_id: u64,
    conversation: Conversation,
) -> Result<(), String> {
    state
        .service()?
        .push(pairing_id, &conversation)
        .await
        .map_err(|e| e.to_string())
}

/// 用户确认入站配对码一致后，接收对端推送的对话（结果经 `net-event` 事件抛出）。
#[tauri::command]
pub async fn accept_incoming(state: State<'_, AppState>, pairing_id: u64) -> Result<(), String> {
    state
        .service()?
        .accept_incoming(pairing_id)
        .await
        .map_err(|e| e.to_string())
}

/// 拒绝/取消一个待确认配对。
#[tauri::command]
pub fn reject_pairing(state: State<'_, AppState>, pairing_id: u64) {
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

/// 设置关闭窗口时是否最小化到托盘。
#[tauri::command]
pub fn set_minimize_to_tray(state: State<'_, AppState>, enabled: bool) {
    *state.minimize_to_tray.lock().unwrap() = enabled;
}
