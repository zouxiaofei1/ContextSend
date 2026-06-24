//! ContextSend 网络层（cs-network）。
//!
//! 负责局域网设备发现（mDNS）、设备配对（6 位配对码 + 密钥交换）、
//! 以及对话上下文的 AES-GCM 加密传输。
//!
//! Phase 0 仅定义核心数据结构占位；mDNS / 配对 / 加密传输在 Phase 1 实现。

use serde::{Deserialize, Serialize};

/// 局域网中被发现的一台设备。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// 配对后生成的稳定 UUID（Phase 1 起持久化，用于设备记忆与重连）。
    pub id: String,
    /// 展示名称（默认随机词组，可由用户改名）。
    pub name: String,
    /// 是否在线（Phase 1 起实时监测）。
    pub online: bool,
}

/// 网络层错误类型。
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("尚未实现")]
    NotImplemented,
}

/// 启动 mDNS 设备发现。Phase 0 占位，返回未实现错误。
pub fn start_discovery() -> Result<Vec<Device>, NetworkError> {
    Err(NetworkError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_serializes() {
        let d = Device {
            id: "uuid-1".into(),
            name: "晨雾微风".into(),
            online: true,
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("晨雾微风"));
    }
}
