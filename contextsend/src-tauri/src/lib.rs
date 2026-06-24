//! ContextSend 桌面应用入口（Tauri v2 外壳）。
//!
//! 本 crate 负责 UI 外壳：装配窗口、托盘、IPC command，并桥接到分层 crate
//! （[`cs_core`] / `cs_adapters` / `cs_network`）。业务逻辑在各 crate 内实现。

mod commands;
mod tray;

use std::sync::Mutex;

use commands::AppState;
use cs_network::{DeviceIdentity, NetworkService};
use tauri::{Emitter, Manager, WindowEvent};

/// 应用主入口，由 `main.rs`（桌面）调用。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .setup(|app| {
            tray::setup_tray(app)?;

            // 身份持久化到 app 数据目录。
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let identity_path = dir.join("identity.json");
            let identity = DeviceIdentity::load_or_create(&identity_path)?;

            // 启动网络服务（异步），并把事件转发到前端。
            let (service, mut events_rx) =
                tauri::async_runtime::block_on(NetworkService::start(identity.clone()))
                    .map_err(|e| std::io::Error::other(e.to_string()))?;

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = events_rx.recv().await {
                    let _ = handle.emit("net-event", event);
                }
            });

            app.manage(AppState {
                service,
                identity_path,
                identity: Mutex::new(identity),
                minimize_to_tray: Mutex::new(true),
            });

            // 拦截窗口关闭事件：若 minimize_to_tray 为 true 则隐藏到托盘
            let window = app.get_webview_window("main").unwrap();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let state = app_handle.state::<AppState>();
                    let minimize = *state.minimize_to_tray.lock().unwrap();
                    if minimize {
                        api.prevent_close();
                        if let Some(w) = app_handle.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_self_identity,
            commands::rename_self,
            commands::list_devices,
            commands::connect_pair,
            commands::push_conversation,
            commands::accept_incoming,
            commands::reject_pairing,
            commands::import_openai,
            commands::export_openai,
            commands::set_minimize_to_tray,
        ])
        .run(tauri::generate_context!())
        .expect("启动 ContextSend 失败");
}
