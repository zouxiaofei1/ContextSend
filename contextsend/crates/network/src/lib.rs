//! ContextSend 网络层（cs-network）。
//!
//! 负责局域网设备发现（mDNS）、设备配对（6 位配对码 SAS + X25519 密钥交换）、
//! 以及对话上下文的 AES-256-GCM 加密传输。
//!
//! 模块划分：
//! - [`identity`]：稳定 UUID + 显示名，磁盘持久化。
//! - [`naming`]：随机中文设备名。
//! - [`crypto`]：ECDH / HKDF / SAS / AES-256-GCM。
//! - [`wire`]：分帧与消息定义。
//! - [`pairing`]：握手与加密会话。
//! - [`discovery`]：mDNS 广播与浏览。

pub mod crypto;
pub mod discovery;
pub mod identity;
pub mod naming;
pub mod pairing;
pub mod service;
pub mod wire;

pub use discovery::{DiscoveredDevice, Discovery, DiscoveryEvent};
pub use identity::DeviceIdentity;
pub use pairing::{handshake, LocalHello, PeerInfo, Session};
pub use service::{NetEvent, NetworkService};
pub use wire::AppMessage;

use serde::{Deserialize, Serialize};

/// 面向 UI 的设备视图（设备列表项）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// 设备 UUID（用于设备记忆与重连）。
    pub id: String,
    /// 展示名称（默认随机词组，可由用户改名）。
    pub name: String,
    /// 是否在线（mDNS 在线即 true）。
    pub online: bool,
}

/// 网络层统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("协议错误: {0}")]
    Protocol(String),
    #[error("加解密错误: {0}")]
    Crypto(String),
    #[error("mDNS 错误: {0}")]
    Mdns(String),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
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
