//! 网络服务编排层。
//!
//! 把发现、监听、配对、推送串成一个供 UI（src-tauri）驱动的高层 API：
//! - 后台运行 mDNS 发现 + TCP 监听；
//! - 主动配对：连接对端 → 握手 → 返回 6 位配对码，等用户确认后推送；
//! - 被动配对：接受连接 → 握手 → 抛出配对码事件，等用户确认后接收。
//!
//! 配对码采用 SAS：双方各自从同一 ECDH 共享密钥派生出同一个 6 位码，用户两端比对一致即可。

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use cs_core::Conversation;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::discovery::{Discovery, DiscoveryEvent};
use crate::identity::DeviceIdentity;
use crate::pairing::{handshake, LocalHello, Session};
use crate::wire::AppMessage;
use crate::{Device, NetworkError};

/// 服务向 UI 抛出的事件。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NetEvent {
    /// 发现（或更新）一台设备。
    DeviceFound(Device),
    /// 一台设备失联。
    DeviceLost { uuid: String },
    /// 收到入站配对请求，附 6 位配对码，等用户确认。
    // 注：外层 `rename_all` 只改变体名，不改变体内字段名，故每个含字段的变体需单独标注。
    #[serde(rename_all = "camelCase")]
    IncomingPairing {
        pairing_id: u64,
        peer_uuid: String,
        peer_name: String,
        pin: String,
    },
    /// 成功接收到一段对话。
    #[serde(rename_all = "camelCase")]
    ConversationReceived {
        from_uuid: String,
        from_name: String,
        conversation: Conversation,
    },
    /// 某次配对/传输失败。
    #[serde(rename_all = "camelCase")]
    Failed { pairing_id: u64, reason: String },
}

struct Inner {
    /// 已发现设备：uuid -> (name, addr)。
    devices: Mutex<HashMap<String, crate::discovery::DiscoveredDevice>>,
    /// 等待用户确认的会话：pairing_id -> Session。
    pending: Mutex<HashMap<u64, Session<TcpStream>>>,
    next_id: AtomicU64,
    events: UnboundedSender<NetEvent>,
    /// 本机身份（可改名）。握手与广播都读这一份，保证改名实时生效。
    identity: Mutex<DeviceIdentity>,
    /// mDNS 发现器，用于改名重新广播与退出注销。
    discovery: Discovery,
}

impl Inner {
    /// 构造当前身份的握手 Hello（每次读实时名）。
    fn local_hello(&self) -> LocalHello {
        let id = self.identity.lock().unwrap();
        LocalHello {
            uuid: id.uuid.clone(),
            name: id.name.clone(),
        }
    }
}

/// 网络服务句柄。克隆廉价（内部 `Arc`）。
#[derive(Clone)]
pub struct NetworkService {
    inner: Arc<Inner>,
}

impl NetworkService {
    /// 启动服务：绑定 TCP 监听 → 启动 mDNS → 运行后台循环。返回服务与事件接收端。
    pub async fn start(
        identity: DeviceIdentity,
    ) -> Result<(Self, UnboundedReceiver<NetEvent>), NetworkError> {
        // 先绑定监听，拿到真实端口再向 mDNS 广播。
        let listener = TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| NetworkError::Io(e.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|e| NetworkError::Io(e.to_string()))?
            .port();
        log::info!("TCP 监听已绑定: 0.0.0.0:{port}");

        let (events_tx, events_rx) = unbounded_channel();

        // 启动 mDNS 发现。
        let (discovery, mut disc_rx) = Discovery::start(&identity, port)?;

        let inner = Arc::new(Inner {
            devices: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            events: events_tx,
            identity: Mutex::new(identity),
            discovery,
        });

        // 后台：发现事件 → 设备表 + UI 事件。
        {
            let inner = Arc::clone(&inner);
            tokio::spawn(async move {
                while let Some(event) = disc_rx.recv().await {
                    match event {
                        DiscoveryEvent::Found(dev) => {
                            log::debug!("发现设备: name={} uuid={}", dev.name, dev.uuid);
                            let ip = primary_ip(&dev.addrs);
                            let os = dev.os.clone();
                            inner
                                .devices
                                .lock()
                                .unwrap()
                                .insert(dev.uuid.clone(), dev.clone());
                            let _ = inner.events.send(NetEvent::DeviceFound(Device {
                                id: dev.uuid,
                                name: dev.name,
                                online: true,
                                os,
                                ip,
                            }));
                        }
                        DiscoveryEvent::Lost { uuid } => {
                            log::debug!("设备失联: uuid={uuid}");
                            inner.devices.lock().unwrap().remove(&uuid);
                            let _ = inner.events.send(NetEvent::DeviceLost { uuid });
                        }
                    }
                }
            });
        }

        // 后台：接受入站连接 → 握手 → 抛出配对码。
        {
            let inner = Arc::clone(&inner);
            tokio::spawn(async move {
                loop {
                    let (stream, _peer) = match listener.accept().await {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    log::debug!("接受入站连接: peer={_peer}");
                    let inner = Arc::clone(&inner);
                    // 每次连接读实时身份，保证改名后握手发的是新名。
                    let local = inner.local_hello();
                    tokio::spawn(async move {
                        match handshake(stream, &local).await {
                            Ok(session) => {
                                let pairing_id = inner.next_id.fetch_add(1, Ordering::Relaxed);
                                let peer_uuid = session.peer.uuid.clone();
                                let peer_name = session.peer.name.clone();
                                let pin = session.pin.clone();
                                log::info!(
                                    "入站握手完成，待用户确认: pairing_id={pairing_id} peer={peer_name}({peer_uuid})"
                                );
                                inner.pending.lock().unwrap().insert(pairing_id, session);
                                let _ = inner.events.send(NetEvent::IncomingPairing {
                                    pairing_id,
                                    peer_uuid,
                                    peer_name,
                                    pin,
                                });
                            }
                            Err(e) => {
                                log::warn!("入站握手失败: {e}");
                                let _ = inner.events.send(NetEvent::Failed {
                                    pairing_id: 0,
                                    reason: e.to_string(),
                                });
                            }
                        }
                    });
                }
            });
        }

        let service = Self { inner };
        Ok((service, events_rx))
    }

    /// 本机身份快照（含当前显示名）。
    pub fn identity(&self) -> DeviceIdentity {
        self.inner.identity.lock().unwrap().clone()
    }

    /// 改名：更新本机身份并立即重新广播 mDNS（更新 TXT 中的 `name`），
    /// 使对端无需等缓存过期即可看到新名；后续握手也会用新名。
    pub fn rename(&self, new_name: &str) -> Result<(), NetworkError> {
        {
            let mut id = self.inner.identity.lock().unwrap();
            id.name = new_name.to_string();
        }
        self.inner.discovery.update_name(new_name)
    }

    /// 主动注销 mDNS（发送 goodbye）并停止发现。退出前调用，让对端立即移除本机。
    pub fn shutdown(&self) {
        self.inner.discovery.shutdown();
    }

    /// 当前设备列表快照。
    pub fn list_devices(&self) -> Vec<Device> {
        self.inner
            .devices
            .lock()
            .unwrap()
            .values()
            .map(|d| Device {
                id: d.uuid.clone(),
                name: d.name.clone(),
                online: true,
                os: d.os.clone(),
                ip: primary_ip(&d.addrs),
            })
            .collect()
    }

    /// 主动向目标设备发起配对：连接 + 握手，返回 `(pairing_id, 6 位配对码)`。
    ///
    /// 调用方应把配对码显示给用户，比对一致后再调用 [`Self::push`]。
    pub async fn connect_pair(&self, target_uuid: &str) -> Result<(u64, String), NetworkError> {
        let addrs = self
            .inner
            .devices
            .lock()
            .unwrap()
            .get(target_uuid)
            .map(|d| d.addrs.clone())
            .ok_or_else(|| NetworkError::Protocol("目标设备不在设备列表中".into()))?;

        // 对端可能广播了多块网卡（含虚拟网卡）的地址，逐个尝试，带超时避免卡在
        // 不可达的虚拟子网上。
        let mut last_err = NetworkError::Protocol("目标设备没有可用地址".into());
        let mut stream = None;
        for addr in &addrs {
            log::debug!("尝试连接目标地址: {addr}");
            match tokio::time::timeout(std::time::Duration::from_secs(3), TcpStream::connect(addr))
                .await
            {
                Ok(Ok(s)) => {
                    log::debug!("已连接目标地址: {addr}");
                    stream = Some(s);
                    break;
                }
                Ok(Err(e)) => {
                    log::debug!("{addr} 连接失败: {e}");
                    last_err = NetworkError::Io(format!("{addr} 连接失败: {e}"));
                }
                Err(_) => {
                    log::debug!("{addr} 连接超时");
                    last_err = NetworkError::Io(format!("{addr} 连接超时"));
                }
            }
        }
        let stream = stream.ok_or(last_err)?;

        let local = self.inner.local_hello();
        let session = handshake(stream, &local).await?;
        let pin = session.pin.clone();
        let pairing_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        log::info!(
            "主动握手完成: pairing_id={pairing_id} peer={}({})",
            session.peer.name,
            session.peer.uuid
        );
        self.inner
            .pending
            .lock()
            .unwrap()
            .insert(pairing_id, session);
        Ok((pairing_id, pin))
    }

    /// 用户确认配对码一致后，把对话推送给已配对会话。完成后会话关闭。
    pub async fn push(
        &self,
        pairing_id: u64,
        conversation: &Conversation,
    ) -> Result<(), NetworkError> {
        let mut session = self.take_pending(pairing_id)?;
        let msg_count = conversation.messages.len();
        log::debug!("开始推送对话: pairing_id={pairing_id} messages={msg_count}");
        session.send_conversation(conversation).await?;
        // 等待对端 Ack（尽力而为）。
        let _ = session.recv().await;
        log::debug!("对话已发送并等待 Ack 完成: pairing_id={pairing_id} messages={msg_count}");
        Ok(())
    }

    /// 用户确认入站配对码一致后，接收对端推送的对话并抛出事件。
    pub async fn accept_incoming(&self, pairing_id: u64) -> Result<(), NetworkError> {
        let mut session = self.take_pending(pairing_id)?;
        let from_uuid = session.peer.uuid.clone();
        let from_name = session.peer.name.clone();
        match session.recv().await? {
            AppMessage::PushConversation(conversation) => {
                let _ = session.send(&AppMessage::Ack).await;
                log::info!(
                    "收到对话: from={from_name}({from_uuid}) messages={}",
                    conversation.messages.len()
                );
                let _ = self.inner.events.send(NetEvent::ConversationReceived {
                    from_uuid,
                    from_name,
                    conversation,
                });
                Ok(())
            }
            AppMessage::Ack => Err(NetworkError::Protocol("期望对话，收到 Ack".into())),
        }
    }

    /// 拒绝/取消一个待确认会话。
    pub fn reject(&self, pairing_id: u64) {
        let _ = self.take_pending(pairing_id);
    }

    fn take_pending(&self, pairing_id: u64) -> Result<Session<TcpStream>, NetworkError> {
        self.inner
            .pending
            .lock()
            .unwrap()
            .remove(&pairing_id)
            .ok_or_else(|| NetworkError::Protocol("配对会话不存在或已过期".into()))
    }
}

/// 从对端广播的候选地址中挑一个用于 UI 展示的首选 IP：
/// 优先非回环的 IPv4（最贴近用户认知的局域网地址），否则退回第一个地址；
/// 无地址时返回空串。
fn primary_ip(addrs: &[SocketAddr]) -> String {
    addrs
        .iter()
        .find(|a| a.is_ipv4() && !a.ip().is_loopback())
        .or_else(|| addrs.iter().find(|a| !a.ip().is_loopback()))
        .or_else(|| addrs.first())
        .map(|a| a.ip().to_string())
        .unwrap_or_default()
}
