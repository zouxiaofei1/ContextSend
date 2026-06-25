//! 局域网设备发现（mDNS）。
//!
//! 每个实例在 `_contextsend._tcp.local.` 下注册自身（TXT 记录携带 `uuid` / `name`），
//! 同时浏览同类服务，发现/失联事件经 tokio 通道上抛给上层。

use std::net::SocketAddr;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::identity::DeviceIdentity;
use crate::NetworkError;

/// ContextSend 的 mDNS 服务类型。
pub const SERVICE_TYPE: &str = "_contextsend._tcp.local.";

/// 一台被发现的设备（来自 mDNS 解析）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub uuid: String,
    pub name: String,
    /// 对端广播的所有候选地址（多网卡时含虚拟网卡），连接时逐个尝试。
    pub addrs: Vec<SocketAddr>,
}

/// 发现层向上层推送的事件。
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// 发现（或更新）一台设备。
    Found(DiscoveredDevice),
    /// 一台设备失联。
    Lost { uuid: String },
}

/// 运行中的发现器，持有 mDNS 守护进程句柄。
pub struct Discovery {
    daemon: ServiceDaemon,
}

impl Discovery {
    /// 启动广播 + 浏览。返回发现器与事件接收端。
    ///
    /// `port` 是本机配对 TCP 监听端口，写入 mDNS 以便对端连接。
    pub fn start(
        identity: &DeviceIdentity,
        port: u16,
    ) -> Result<(Self, UnboundedReceiver<DiscoveryEvent>), NetworkError> {
        let daemon = ServiceDaemon::new().map_err(|e| NetworkError::Mdns(e.to_string()))?;

        // 注册本机服务。实例名用 UUID 保证唯一；host 用 UUID.local.，地址自动探测。
        let host = format!("{}.local.", identity.uuid);
        let props = [
            ("uuid", identity.uuid.as_str()),
            ("name", identity.name.as_str()),
        ];
        let info = ServiceInfo::new(SERVICE_TYPE, &identity.uuid, &host, "", port, &props[..])
            .map_err(|e| NetworkError::Mdns(e.to_string()))?
            .enable_addr_auto();
        daemon
            .register(info)
            .map_err(|e| NetworkError::Mdns(e.to_string()))?;

        // 浏览同类服务。
        let browse_rx = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| NetworkError::Mdns(e.to_string()))?;

        let (tx, rx) = unbounded_channel();
        let self_uuid = identity.uuid.clone();

        // mdns-sd 的事件通道是阻塞式的，放到独立线程里泵到 tokio 通道。
        std::thread::spawn(move || {
            while let Ok(event) = browse_rx.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let uuid = info
                            .get_property_val_str("uuid")
                            .map(str::to_string)
                            .unwrap_or_else(|| instance_of(info.get_fullname()));
                        // 忽略自己。
                        if uuid == self_uuid {
                            continue;
                        }
                        let name = info
                            .get_property_val_str("name")
                            .map(str::to_string)
                            .unwrap_or_else(|| uuid.clone());
                        let port = info.get_port();
                        let addrs: Vec<SocketAddr> = info
                            .get_addresses()
                            .iter()
                            .map(|ip| SocketAddr::new(*ip, port))
                            .collect();
                        if !addrs.is_empty() {
                            let device = DiscoveredDevice { uuid, name, addrs };
                            if tx.send(DiscoveryEvent::Found(device)).is_err() {
                                break; // 上层已关闭。
                            }
                        }
                    }
                    ServiceEvent::ServiceRemoved(_ty, fullname) => {
                        let uuid = instance_of(&fullname);
                        if uuid == self_uuid {
                            continue;
                        }
                        if tx.send(DiscoveryEvent::Lost { uuid }).is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok((Self { daemon }, rx))
    }

    /// 停止广播与浏览。
    pub fn shutdown(&self) {
        let _ = self.daemon.shutdown();
    }
}

/// 从 mDNS 全名 `<instance>._contextsend._tcp.local.` 中取出实例名（即 UUID）。
fn instance_of(fullname: &str) -> String {
    fullname.split('.').next().unwrap_or(fullname).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_of_extracts_uuid() {
        let full = "abc-123._contextsend._tcp.local.";
        assert_eq!(instance_of(full), "abc-123");
    }
}
