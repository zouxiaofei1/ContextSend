//! Tauri IPC command 骨架。
//!
//! Phase 0 仅提供一个 [`get_app_info`]，用于验证「Vue(Pinia) → invoke → Rust」通信闭环。

use serde::Serialize;

/// 返回给前端的应用基础信息。
#[derive(Debug, Serialize)]
pub struct AppInfo {
    /// 应用版本（取自编译期 Cargo 版本）。
    pub version: String,
    /// 运行平台标识（windows / macos / linux ...）。
    pub platform: String,
    /// 内置适配器名称列表（来自 [`cs_adapters`]）。
    pub adapters: Vec<String>,
}

/// 获取应用信息。供前端挂载时调用，验证 IPC 通路。
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
