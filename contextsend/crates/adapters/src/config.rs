//! 适配器用户配置：每个适配器的目录 / 端口覆盖项，持久化到 `adapters.json`。
//!
//! 自动探测的默认值（数据目录、ChatBox 的 CDP 端口）可被用户在「设置 → 适配器」里
//! 覆盖。覆盖项以适配器名（小写）为键存进一个进程内全局表，并落盘到应用数据目录下的
//! `adapters.json`。适配器逻辑（[`crate::jan`] / [`crate::chatbox`]）读取此处的覆盖，
//! 未设置时回退到各自的探测默认值。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// 单个适配器的可配置覆盖项。字段为 `None` 表示沿用探测默认值。
///
/// 来自前端的 JSON 用 camelCase（`dataDir` / `installDir` / `port`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterOverride {
    /// 数据目录覆盖（绝对路径）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
    /// 程序安装目录覆盖（绝对路径）。当前仅持久化保存，后端暂未消费。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_dir: Option<String>,
    /// 端口覆盖（如 ChatBox 的 CDP 调试端口）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

/// 全局配置状态：落盘路径 + 各适配器覆盖项（键为适配器名小写）。
struct ConfigState {
    path: PathBuf,
    overrides: HashMap<String, AdapterOverride>,
}

static CONFIG: RwLock<Option<ConfigState>> = RwLock::new(None);

/// 初始化适配器配置：指定 `adapters.json` 路径并从磁盘载入已有覆盖。
///
/// 应在应用启动、且任何适配器读写之前调用一次。文件不存在或损坏时以空配置起步。
pub fn init(path: PathBuf) {
    let overrides = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<HashMap<String, AdapterOverride>>(&s).ok())
        .unwrap_or_default();
    let mut guard = CONFIG.write().unwrap();
    *guard = Some(ConfigState { path, overrides });
}

/// 取某适配器（名字大小写不敏感）的覆盖项；未配置返回默认（全 `None`）。
pub fn get(name: &str) -> AdapterOverride {
    let key = name.to_ascii_lowercase();
    CONFIG
        .read()
        .unwrap()
        .as_ref()
        .and_then(|c| c.overrides.get(&key).cloned())
        .unwrap_or_default()
}

/// 写入某适配器（名字大小写不敏感）的覆盖项并落盘。
///
/// 覆盖项整体替换该适配器的旧值。未初始化（[`init`] 未调用）时返回错误。
pub fn set(name: &str, ov: AdapterOverride) -> std::io::Result<()> {
    let key = name.to_ascii_lowercase();
    let mut guard = CONFIG.write().unwrap();
    let state = guard.as_mut().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "适配器配置未初始化")
    })?;
    state.overrides.insert(key, ov);
    let json = serde_json::to_string_pretty(&state.overrides)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&state.path, json)
}
