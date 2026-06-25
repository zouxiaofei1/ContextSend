//! ContextSend 桌面应用入口（Tauri v2 外壳）。
//!
//! 本 crate 负责 UI 外壳：装配窗口、托盘、IPC command，并桥接到分层 crate
//! （[`cs_core`] / `cs_adapters` / `cs_network`）。业务逻辑在各 crate 内实现。

mod commands;
mod tray;

use std::collections::HashMap;
use std::sync::Mutex;

use commands::AppState;
use cs_network::{DeviceIdentity, NetEvent, NetworkService};
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tokio::sync::OnceCell;

/// 应用主入口，由 `main.rs`（桌面）调用。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(build_log_plugin())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .setup(|app| {
            tray::setup_tray(app)?;

            // 身份持久化到 app 数据目录。
            // 调试用：设置 CONTEXTSEND_DATA_DIR 可覆盖数据目录，便于同机开多个实例
            // （不同目录 → 不同 identity.json → 不同 UUID → 能互相发现/配对）。
            let dir = match std::env::var_os("CONTEXTSEND_DATA_DIR") {
                Some(custom) => std::path::PathBuf::from(custom),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&dir)?;
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
                            let mut online_devices: HashMap<String, String> =
                                HashMap::new();
                            while let Some(event) = events_rx.recv().await {
                                // 跟踪在线设备变化
                                let mut changed = false;
                                match &event {
                                    NetEvent::DeviceFound(device) => {
                                        if device.online {
                                            online_devices.insert(
                                                device.id.clone(),
                                                device.name.clone(),
                                            );
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
                                        .map(|(id, name)| {
                                            (id.clone(), name.clone())
                                        })
                                        .collect();
                                    // 写入 AppState 供托盘菜单读取
                                    if let Some(state) =
                                        tray_handle.try_state::<AppState>()
                                    {
                                        *state.online_devices.lock().unwrap() =
                                            snapshot;
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
            commands::set_minimize_to_tray,
        ])
        .run(tauri::generate_context!())
        .expect("启动 ContextSend 失败");
}

/// 构造日志插件：同时输出到终端(开发)与日志文件(`app_log_dir/ContextSend.log`)。
///
/// 级别策略：本应用各 crate（`contextsend_lib` / `cs_network` / `cs_adapters` /
/// `cs_core`）默认 `Debug`，第三方依赖（mDNS、tungstenite 等）压到 `Warn`，
/// 避免底层库刷屏淹没业务日志。隐私：业务埋点只记元信息（条数/字节/耗时/设备名/
/// 应用名/UUID），不记录对话正文与配对码。
fn build_log_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

    tauri_plugin_log::Builder::new()
        // 时间戳用本地时区（默认是 UTC，会与本地时间差几个小时）。
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .targets([
            // 终端输出：开发期实时查看。
            Target::new(TargetKind::Stdout),
            // 文件输出：滚动写入 app 日志目录，便于事后排查（双机调试时各实例独立）。
            Target::new(TargetKind::LogDir {
                file_name: Some("ContextSend".into()),
            }),
        ])
        // 全局默认 Info；本项目 crate 提到 Debug；噪声大的底层库压到 Warn。
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
