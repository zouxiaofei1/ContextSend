//! 系统托盘装配。
//!
//! 提供托盘图标、右键菜单（展示在线设备 / 开机自启 / 显示窗口 / 退出），
//! 以及左键点击切换主窗口显隐。
//! 菜单会在设备变化或开机自启切换时动态重建。

use crate::commands::AppState;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_autostart::ManagerExt as _;

/// 在应用启动时创建并注册系统托盘。
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("ContextSend 上下文分享")
        .icon(app.default_window_icon().expect("应已配置应用图标").clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_and_focus(app),
            "quit" => app.exit(0),
            "toggle_autostart" => {
                // 切换开机自启并刷新菜单
                let mgr = app.autolaunch();
                let current = mgr.is_enabled().unwrap_or(false);
                if current {
                    let _ = mgr.disable();
                } else {
                    let _ = mgr.enable();
                }
                // 同步 AppState
                if let Some(state) = app.try_state::<AppState>() {
                    *state.auto_start.lock().unwrap() = !current;
                }
                update_menu(app);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键松开：切换主窗口显隐。
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        show_and_focus(app);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// 动态重建托盘菜单。
///
/// 从 [`AppState`] 读取在线设备列表与开机自启状态，
/// 构建完整菜单并替换当前菜单。
pub fn update_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let Some(tray) = app.tray_by_id("main-tray") else {
        return;
    };

    let (auto_start, online_devices) = {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let auto = *state.auto_start.lock().unwrap();
        let devices = state.online_devices.lock().unwrap().clone();
        (auto, devices)
    };

    let count = online_devices.len();
    let tooltip = if count == 0 {
        "ContextSend 上下文分享".to_string()
    } else {
        format!("ContextSend — {} 台设备在线", count)
    };
    let _ = tray.set_tooltip(Some(&tooltip));

    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<R>>> = Vec::new();

    // 在线设备（disabled 展示项）
    if count > 0 {
        let header = format!("在线设备 ({})", count);
        if let Ok(item) =
            MenuItem::with_id(app, "tray_devices_header", &header, false, None::<&str>)
        {
            items.push(Box::new(item));
        }
        for (id, name) in &online_devices {
            let item_id = format!("tray_dev_{}", id);
            let display = truncate_name(name, 32);
            if let Ok(item) = MenuItem::with_id(app, item_id, &display, false, None::<&str>) {
                items.push(Box::new(item));
            }
        }
    }

    // 开机自启（可切换 CheckMenuItem）
    if let Ok(item) = CheckMenuItem::with_id(
        app,
        "toggle_autostart",
        "开机自启",
        true,
        auto_start,
        None::<&str>,
    ) {
        items.push(Box::new(item));
    }

    // 显示窗口
    if let Ok(item) = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>) {
        items.push(Box::new(item));
    }

    // 退出
    if let Ok(item) = MenuItem::with_id(app, "quit", "退出", true, None::<&str>) {
        items.push(Box::new(item));
    }

    let refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = items.iter().map(|b| b.as_ref()).collect();
    if let Ok(menu) = Menu::with_items(app, &refs) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// 显示主窗口并置于前台。
fn show_and_focus<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 按 Unicode 字符数截断名称，超出 max_len 则追加 "…"。
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() > max_len {
        name.chars().take(max_len).collect::<String>() + "\u{2026}"
    } else {
        name.to_string()
    }
}
