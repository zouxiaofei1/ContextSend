//! 局域网设备发现（mDNS）。
//!
//! 每个实例在 `_contextsend._tcp.local.` 下注册自身（TXT 记录携带 `uuid` / `name`），
//! 同时浏览同类服务，发现/失联事件经 tokio 通道上抛给上层。
//!
//! 可靠性设计：
//! - **改名实时生效**：[`Discovery::update_name`] 以相同实例名重新注册，更新 TXT 并触发
//!   重新公告，对端无需等缓存过期即可看到新名。
//! - **周期性重新公告**：后台线程每 [`REANNOUNCE_INTERVAL`] 重新注册一次，弥补多播 UDP
//!   丢包导致的「单向可见」（A 见 B、B 不见 A）。
//! - **退出主动注销**：[`Discovery::shutdown`] 发送 mDNS goodbye，让对端立即移除本机，
//!   而非苦等 TTL 超时。

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::identity::DeviceIdentity;
use crate::identity::{self, NAME_MAX_LEN};
use crate::NetworkError;

/// ContextSend 的 mDNS 服务类型。
pub const SERVICE_TYPE: &str = "_contextsend._tcp.local.";

/// 周期性重新公告间隔：弥补多播丢包，让两端持续收敛。
pub const REANNOUNCE_INTERVAL: Duration = Duration::from_secs(30);

/// 一台被发现的设备（来自 mDNS 解析）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub uuid: String,
    pub name: String,
    /// 对端操作系统标识（`std::env::consts::OS`：windows / linux / macos…），用于 UI 展示平台图标。
    pub os: String,
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

/// 运行中的发现器，持有 mDNS 守护进程句柄与当前广播参数。
pub struct Discovery {
    daemon: ServiceDaemon,
    /// 本机 UUID（既是实例名，也是 host 前缀）。
    uuid: String,
    /// 本机配对 TCP 端口。
    port: u16,
    /// 当前广播的显示名，改名时更新（供重新注册读取）。
    name: Arc<Mutex<String>>,
    /// 是否已注销，避免重新公告线程在 shutdown 后继续 register。
    closed: Arc<AtomicBool>,
}

/// 构造本机 [`ServiceInfo`]：实例名用 UUID 保证唯一；host 用 `UUID.local.`，地址自动探测。
fn build_service_info(
    uuid: &str,
    name: &str,
    port: u16,
) -> Result<ServiceInfo, NetworkError> {
    let host = format!("{uuid}.local.");
    // 额外广播本机操作系统，供对端 UI 展示平台（win/linux/mac）图标。
    let props = [("uuid", uuid), ("name", name), ("os", std::env::consts::OS)];
    ServiceInfo::new(SERVICE_TYPE, uuid, &host, "", port, &props[..])
        .map(|info| info.enable_addr_auto())
        .map_err(|e| NetworkError::Mdns(e.to_string()))
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

        // 注册本机服务。
        let info = build_service_info(&identity.uuid, &identity.name, port)?;
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
                            .map(|s| identity::truncate_name(s, NAME_MAX_LEN))
                            .unwrap_or_else(|| uuid.clone());
                        let os = info
                            .get_property_val_str("os")
                            .map(str::to_string)
                            .unwrap_or_default();
                        let port = info.get_port();
                        let addrs: Vec<SocketAddr> = info
                            .get_addresses()
                            .iter()
                            .map(|ip| SocketAddr::new(*ip, port))
                            .collect();
                        if !addrs.is_empty() {
                            let device = DiscoveredDevice {
                                uuid,
                                name,
                                os,
                                addrs,
                            };
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

        let name = Arc::new(Mutex::new(identity.name.clone()));
        let closed = Arc::new(AtomicBool::new(false));

        // 后台：周期性重新公告自身，弥补多播 UDP 丢包导致的「单向可见」。
        // 重新注册会让 mdns-sd 重发公告，并刷新对端缓存的存活时间。
        {
            let daemon = daemon.clone();
            let uuid = identity.uuid.clone();
            let name = Arc::clone(&name);
            let closed = Arc::clone(&closed);
            std::thread::spawn(move || loop {
                std::thread::sleep(REANNOUNCE_INTERVAL);
                if closed.load(Ordering::Relaxed) {
                    break;
                }
                let current = name.lock().unwrap().clone();
                match build_service_info(&uuid, &current, port) {
                    Ok(info) => {
                        if let Err(e) = daemon.register(info) {
                            log::warn!("周期性重新公告失败: {e}");
                        }
                    }
                    Err(e) => log::warn!("周期性重新公告构造失败: {e}"),
                }
            });
        }

        let discovery = Self {
            daemon,
            uuid: identity.uuid.clone(),
            port,
            name,
            closed,
        };
        Ok((discovery, rx))
    }

    /// 改名后实时更新 mDNS 广播：以相同实例名重新注册，更新 TXT 中的 `name` 并触发
    /// 重新公告，使对端无需等缓存过期即可看到新名。
    pub fn update_name(&self, new_name: &str) -> Result<(), NetworkError> {
        *self.name.lock().unwrap() = new_name.to_string();
        let info = build_service_info(&self.uuid, new_name, self.port)?;
        self.daemon
            .register(info)
            .map_err(|e| NetworkError::Mdns(e.to_string()))?;
        log::debug!("mDNS 已重新注册新名: {new_name}");
        Ok(())
    }

    /// 停止广播与浏览：发送 mDNS goodbye 注销本机服务，让对端立即移除本机。
    pub fn shutdown(&self) {
        self.closed.store(true, Ordering::Relaxed);
        // 主动注销服务（goodbye 包），对端据此立即下线本机，而非等待 TTL 超时。
        let fullname = format!("{}.{}", self.uuid, SERVICE_TYPE);
        let _ = self.daemon.unregister(&fullname);
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

    #[test]
    fn update_name_and_shutdown_do_not_error() {
        // 启动一个真实 mDNS 守护，验证改名重新注册与退出注销不报错、不 panic。
        // 不依赖网络对端，仅覆盖本机注册/注销路径。
        let id = DeviceIdentity::generate();
        let (discovery, _rx) = Discovery::start(&id, 12345).expect("启动发现器");

        discovery.update_name("新名字").expect("改名重新注册应成功");
        assert_eq!(*discovery.name.lock().unwrap(), "新名字");

        // 注销后内部标志应置位，重新公告线程据此退出。
        discovery.shutdown();
        assert!(discovery.closed.load(Ordering::Relaxed));
    }
}
