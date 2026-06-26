//! ContextSend 桌面应用入口（Tauri v2 外壳）。
//!
//! 本 crate 负责 UI 外壳：装配窗口、托盘、IPC command，并桥接到分层 crate
//! （[`cs_core`] / `cs_adapters` / `cs_network`）。业务逻辑在各 crate 内实现。

mod commands;
mod shortcut;
mod tray;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use commands::AppState;
use cs_network::{DeviceIdentity, NetEvent, NetworkService};
use tauri::{Emitter, Manager, WebviewUrl, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tokio::sync::OnceCell;

/// 应用数据根目录：所有持久化文件（identity、store、日志、WebView2 缓存）
/// 统一存放于 `%LOCALAPPDATA%\ContextSend\`（Windows）或对应平台 local data 目录。
///
/// 调试用：设置 `CONTEXTSEND_DATA_DIR` 可覆盖，便于同机多实例。
pub fn data_root() -> PathBuf {
    match std::env::var_os("CONTEXTSEND_DATA_DIR") {
        Some(custom) => PathBuf::from(custom),
        None => dirs::data_local_dir()
            .expect("无法获取系统 LocalAppData 目录")
            .join("ContextSend"),
    }
}

/// 应用主入口，由 `main.rs`（桌面）调用。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let root = data_root();
    std::fs::create_dir_all(&root).expect("无法创建数据目录");

    // 日志插件需要在 Builder 之前构建（需要 root 路径）。
    let log_plugin = build_log_plugin(&root);

    tauri::Builder::default()
        .plugin(log_plugin)
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .setup(|app| {
            tray::setup_tray(app)?;

            // 编程创建主窗口，显式指定 WebView2 数据目录到统一的 data_root()。
            let dir = data_root();
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::default(),
            )
            .title("ContextSend 上下文分享")
            .inner_size(960.0, 680.0)
            .min_inner_size(300.0, 500.0)
            .resizable(true)
            .decorations(false)
            .visible(false)
            .drag_and_drop(false)
            .data_directory(dir.clone())
            .build()?;

            // 全局快捷键插件（仅桌面端）：注册「显示/隐藏主窗口」回调。
            // 具体热键由前端启动时经 set_global_shortcut 动态注册。
            #[cfg(desktop)]
            app.handle().plugin(shortcut::init_plugin())?;

            migrate_legacy_data(app, &dir);

            let identity_path = dir.join("identity.json");
            let identity = DeviceIdentity::load_or_create(&identity_path)?;
            log::info!(
                "本机身份就绪: name={} uuid={} data_dir={}",
                identity.name,
                identity.uuid,
                dir.display()
            );

            // 读取当前开机自启状态（OS 级）。
            let auto_start = app.autolaunch().is_enabled().unwrap_or(false);

            // 先注册状态（网络服务尚未就绪），让窗口与前端不被网络初始化阻塞。
            app.manage(AppState {
                service: OnceCell::new(),
                identity_path,
                identity: Mutex::new(identity.clone()),
                minimize_to_tray: Mutex::new(true),
                auto_start: Mutex::new(auto_start),
                online_devices: Mutex::new(Vec::new()),
            });

            // 后台异步启动网络服务：就绪后填充 service 并广播事件，
            // 避免 mDNS 初始化阻塞 setup（首屏可立即呈现）。
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match NetworkService::start(identity).await {
                    Ok((service, mut events_rx)) => {
                        log::info!("网络服务已启动，开始监听 mDNS 与入站连接");
                        // 转发网络事件到前端，同时同步更新 AppState 与托盘菜单。
                        let emit_handle = handle.clone();
                        let tray_handle = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let mut online_devices: HashMap<String, String> = HashMap::new();
                            while let Some(event) = events_rx.recv().await {
                                // 跟踪在线设备变化
                                let mut changed = false;
                                match &event {
                                    NetEvent::DeviceFound(device) => {
                                        if device.online {
                                            online_devices
                                                .insert(device.id.clone(), device.name.clone());
                                            changed = true;
                                        }
                                    }
                                    NetEvent::DeviceLost { uuid } => {
                                        online_devices.remove(uuid);
                                        changed = true;
                                    }
                                    _ => {}
                                }
                                if changed {
                                    let snapshot: Vec<_> = online_devices
                                        .iter()
                                        .map(|(id, name)| (id.clone(), name.clone()))
                                        .collect();
                                    // 写入 AppState 供托盘菜单读取
                                    if let Some(state) = tray_handle.try_state::<AppState>() {
                                        *state.online_devices.lock().unwrap() = snapshot;
                                    }
                                    tray::update_menu(&tray_handle);
                                }
                                let _ = emit_handle.emit("net-event", event);
                            }
                        });
                        let _ = handle.state::<AppState>().service.set(service);
                        let _ = handle.emit("net-ready", ());
                    }
                    Err(e) => {
                        log::error!("网络服务启动失败: {e}");
                        let _ = handle.emit("net-error", e.to_string());
                    }
                }
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
            commands::get_data_dir,
            commands::get_self_identity,
            commands::rename_self,
            commands::list_devices,
            commands::connect_pair,
            commands::push_conversation,
            commands::accept_incoming,
            commands::reject_pairing,
            commands::import_openai,
            commands::export_openai,
            commands::import_to_app,
            commands::match_context,
            commands::save_export,
            commands::set_minimize_to_tray,
            shortcut::set_global_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("启动 ContextSend 失败");
}

/// 构造日志插件：同时输出到终端(开发)与日志文件(`<root>/logs/ContextSend.log`)。
///
/// 级别策略：本应用各 crate（`contextsend_lib` / `cs_network` / `cs_adapters` /
/// `cs_core`）默认 `Debug`，第三方依赖（mDNS、tungstenite 等）压到 `Warn`，
/// 避免底层库刷屏淹没业务日志。隐私：业务埋点只记元信息（条数/字节/耗时/设备名/
/// 应用名/UUID），不记录对话正文与配对码。
fn build_log_plugin<R: tauri::Runtime>(root: &std::path::Path) -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

    tauri_plugin_log::Builder::new()
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Folder {
                path: root.join("logs"),
                file_name: Some("ContextSend".into()),
            }),
        ])
        .level(log::LevelFilter::Info)
        .level_for("contextsend_lib", log::LevelFilter::Debug)
        .level_for("cs_network", log::LevelFilter::Debug)
        .level_for("cs_adapters", log::LevelFilter::Debug)
        .level_for("cs_core", log::LevelFilter::Debug)
        .level_for("mdns_sd", log::LevelFilter::Warn)
        .level_for("tungstenite", log::LevelFilter::Warn)
        .level_for("tokio_tungstenite", log::LevelFilter::Warn)
        .build()
}

/// 从旧版数据目录（`%APPDATA%\dev.contextsend.app`）迁移文件到新目录。
/// 仅在新目录尚无 identity.json 时执行（避免重复迁移）。
fn migrate_legacy_data(app: &tauri::App, new_dir: &std::path::Path) {
    let legacy_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !legacy_dir.exists() || legacy_dir == *new_dir {
        return;
    }
    if new_dir.join("identity.json").exists() {
        return;
    }
    let files = ["identity.json", "segments.json", "permissions.json", "devices.json"];
    for name in &files {
        let src = legacy_dir.join(name);
        if src.exists() {
            if let Err(e) = std::fs::copy(&src, new_dir.join(name)) {
                log::warn!("迁移 {name} 失败: {e}");
            } else {
                log::info!("已迁移: {name}");
            }
        }
    }
}
