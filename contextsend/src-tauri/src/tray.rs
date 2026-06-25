//! 系统托盘装配。
//!
//! 提供托盘图标、右键菜单（显示窗口 / 退出），以及左键点击切换主窗口显隐。
//! 同时支持动态更新托盘菜单以展示当前在线设备列表。

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

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

/// 动态更新托盘菜单以展示当前在线设备。
///
/// `online_devices` 为 `(id, name)` 列表，仅包含在线设备。
/// 会在 tooltip 中显示在线数量，并在菜单中以 disabled 项罗列设备名。
pub fn update_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    online_devices: &[(String, String)],
) {
    let Some(tray) = app.tray_by_id("main-tray") else {
        return;
    };

    let count = online_devices.len();
    let tooltip = if count == 0 {
        "ContextSend 上下文分享".to_string()
    } else {
        format!("ContextSend — {} 台设备在线", count)
    };
    let _ = tray.set_tooltip(Some(&tooltip));

    // 收集菜单项：先放设备名（disabled），再放「显示窗口」「退出」
    let mut items: Vec<MenuItem<R>> = Vec::new();

    if count > 0 {
        let header = format!("在线设备 ({})", count);
        if let Ok(item) =
            MenuItem::with_id(app, "tray_devices_header", &header, false, None::<&str>)
        {
            items.push(item);
        }
        for (id, name) in online_devices {
            let item_id = format!("tray_dev_{}", id);
            if let Ok(item) = MenuItem::with_id(app, item_id, name.as_str(), false, None::<&str>)
            {
                items.push(item);
            }
        }
    }

    if let Ok(show) = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>) {
        items.push(show);
    }
    if let Ok(quit) = MenuItem::with_id(app, "quit", "退出", true, None::<&str>) {
        items.push(quit);
    }

    let refs: Vec<&dyn tauri::menu::IsMenuItem<R>> =
        items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<R>).collect();
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
