//! ContextSend 桌面应用入口（Tauri v2 外壳）。
//!
//! 本 crate 仅负责 UI 外壳：装配窗口、托盘、IPC command，并桥接到分层 crate
//! （[`cs_core`] / [`cs_adapters`] / [`cs_network`]）。业务逻辑不写在这里。

mod commands;
mod tray;

/// 应用主入口，由 `main.rs`（桌面）调用。
///
/// `#[cfg_attr(mobile, tauri::mobile_entry_point)]` 让同一函数可作为未来移动端入口。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 装配系统托盘（图标 + 菜单）。
            tray::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_app_info])
        .run(tauri::generate_context!())
        .expect("启动 ContextSend 失败");
}
