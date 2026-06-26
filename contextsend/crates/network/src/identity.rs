//! 设备身份：稳定 UUID + 显示名，并持久化到磁盘。
//!
//! UUID 在首次运行时生成并写入磁盘，之后保持不变——这是「设备记忆 / 断线重连」的基础。
//! 本模块不决定存储路径（由上层 UI / src-tauri 注入），core/network 不直接依赖具体目录。

use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{naming, NetworkError};

/// 设备名最大字符数（Unicode 标量值，非字节）。
pub const NAME_MAX_LEN: usize = 32;

/// 按 Unicode 字符数截断名称，超出 max_len 则追加 "…"。
pub fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() > max_len {
        name.chars().take(max_len).collect::<String>() + "\u{2026}"
    } else {
        name.to_string()
    }
}

/// 本机设备身份。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    /// 稳定 UUID（持久化后不变）。
    pub uuid: String,
    /// 显示名称（默认随机词组，可改名）。
    pub name: String,
}

impl DeviceIdentity {
    /// 生成一个全新的身份（随机 UUID + 默认设备名）。
    pub fn generate() -> Self {
        Self {
            uuid: Uuid::new_v4().to_string(),
            name: naming::default_device_name(),
        }
    }

    /// 从给定路径加载身份；文件不存在则生成新身份并写入该路径。
    ///
    /// 这是上层获取「本机身份」的推荐入口：保证 UUID 跨重启稳定。
    pub fn load_or_create(path: &Path) -> Result<Self, NetworkError> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let identity: DeviceIdentity = serde_json::from_str(&content)?;
                Ok(identity)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let identity = DeviceIdentity::generate();
                identity.save(path)?;
                Ok(identity)
            }
            Err(e) => Err(NetworkError::Io(e.to_string())),
        }
    }

    /// 将身份写入磁盘（覆盖）。父目录需已存在。
    pub fn save(&self, path: &Path) -> Result<(), NetworkError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json).map_err(|e| NetworkError::Io(e.to_string()))
    }

    /// 改名并写回磁盘。
    /// 校验：非空、不超过 [`NAME_MAX_LEN`] 个字符。
    pub fn rename(&mut self, new_name: impl Into<String>, path: &Path) -> Result<(), NetworkError> {
        let name: String = new_name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(NetworkError::InvalidName("名称不能为空".into()));
        }
        if trimmed.chars().count() > NAME_MAX_LEN {
            return Err(NetworkError::InvalidName(format!(
                "名称不能超过 {} 个字符",
                NAME_MAX_LEN
            )));
        }
        self.name = trimmed.to_string();
        self.save(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_persists_and_is_stable() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cs-identity-test-{}.json", Uuid::new_v4()));

        let first = DeviceIdentity::load_or_create(&path).unwrap();
        let second = DeviceIdentity::load_or_create(&path).unwrap();
        assert_eq!(first, second, "二次加载应得到同一 UUID 与名字");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rename_persists() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cs-identity-rename-{}.json", Uuid::new_v4()));

        let mut id = DeviceIdentity::load_or_create(&path).unwrap();
        id.rename("我的笔记本", &path).unwrap();

        let reloaded = DeviceIdentity::load_or_create(&path).unwrap();
        assert_eq!(reloaded.name, "我的笔记本");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rename_rejects_empty() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cs-identity-empty-{}.json", Uuid::new_v4()));

        let mut id = DeviceIdentity::load_or_create(&path).unwrap();
        let result = id.rename("   ", &path);
        assert!(result.is_err(), "纯空格名应被拒绝");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rename_rejects_too_long() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cs-identity-long-{}.json", Uuid::new_v4()));

        let mut id = DeviceIdentity::load_or_create(&path).unwrap();
        let long_name = "测".repeat(33); // 33 个汉字 > 32
        let result = id.rename(long_name, &path);
        assert!(result.is_err(), "超过 32 字符名应被拒绝");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rename_allows_valid_name() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cs-identity-valid-{}.json", Uuid::new_v4()));

        let mut id = DeviceIdentity::load_or_create(&path).unwrap();
        id.rename("My 笔记本", &path).unwrap();
        assert_eq!(id.name, "My 笔记本");

        let _ = std::fs::remove_file(&path);
    }
}
