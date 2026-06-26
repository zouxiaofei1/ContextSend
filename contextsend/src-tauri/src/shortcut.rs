//! 全局快捷键。
//!
//! 提供「显示/隐藏主窗口」热键：注册逻辑与托盘左键点击共用
//! [`crate::tray::toggle_main_window`]，保证两条入口行为一致。
//!
//! 前端为唯一事实源：用户在「高级设置」里设置/清除热键字符串，
//! 经 [`set_global_shortcut`] 命令同步到后端；启动时也会调用一次完成初始注册。
//!
//! 全局热键是桌面端概念：插件与底层依赖均以 `#[cfg(desktop)]` 编译，
//! 移动端只保留一个空操作的同名命令，使前端调用与 IPC 注册列表无需分平台。

#[cfg(desktop)]
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, ShortcutState,
};

/// 构造全局快捷键插件：注册统一的按下回调（切换主窗口显隐）。
///
/// 具体快捷键不在此处写死——由前端通过 [`set_global_shortcut`] 动态注册；
/// 此回调对所有已注册快捷键生效，只在「按下」时触发，忽略「松开」。
#[cfg(desktop)]
pub fn init_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    GlobalShortcutBuilder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                crate::tray::toggle_main_window(app);
            }
        })
        .build()
}

/// 设置（或清除）「显示/隐藏主窗口」全局快捷键。
///
/// - `accelerator` 为 `Some(非空字符串)`：先清空旧热键再注册新热键；
///   字符串非法或被系统/其他应用占用时返回错误，前端据此提示并回滚。
/// - `accelerator` 为 `None` 或空白：清除热键（功能关闭）。
///
/// 当前仅维护单一全局热键，故直接 `unregister_all` 后再注册，无需追踪旧值。
#[cfg(desktop)]
#[tauri::command]
pub fn set_global_shortcut(
    app: tauri::AppHandle,
    accelerator: Option<String>,
) -> Result<(), String> {
    let gs = app.global_shortcut();

    // 先清空既有注册，避免旧热键残留或重复注册报错。
    gs.unregister_all().map_err(|e| {
        log::warn!("清除旧全局快捷键失败: {e}");
        e.to_string()
    })?;

    let acc = accelerator.unwrap_or_default();
    let acc = acc.trim();
    if acc.is_empty() {
        log::info!("全局快捷键已清除（功能关闭）");
        return Ok(());
    }

    gs.register(acc).map_err(|e| {
        log::warn!("注册全局快捷键失败: accelerator={acc} err={e}");
        e.to_string()
    })?;
    log::info!("全局快捷键已注册: accelerator={acc}");
    Ok(())
}

/// 移动端占位：无全局热键能力，恒成功，使前端调用与命令注册无需分平台。
#[cfg(not(desktop))]
#[tauri::command]
pub fn set_global_shortcut(
    _app: tauri::AppHandle,
    _accelerator: Option<String>,
) -> Result<(), String> {
    Ok(())
}
