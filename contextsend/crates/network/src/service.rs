//! 网络服务编排层。
//!
//! 把发现、监听、配对、推送串成一个供 UI（src-tauri）驱动的高层 API：
//! - 后台运行 mDNS 发现 + TCP 监听；
//! - 主动配对：连接对端 → 握手 → 返回 6 位配对码，等用户确认后推送；
//! - 被动配对：接受连接 → 握手 → 抛出配对码事件，等用户确认后接收。
//!
//! 配对码采用 SAS：双方各自从同一 ECDH 共享密钥派生出同一个 6 位码，用户两端比对一致即可。

use std::collections::HashMap;
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
}

/// 网络服务句柄。克隆廉价（内部 `Arc`）。
#[derive(Clone)]
pub struct NetworkService {
    identity: DeviceIdentity,
    inner: Arc<Inner>,
    _discovery: Arc<Discovery>,
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

        let (events_tx, events_rx) = unbounded_channel();
        let inner = Arc::new(Inner {
            devices: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            events: events_tx,
        });

        // 启动 mDNS 发现。
        let (discovery, mut disc_rx) = Discovery::start(&identity, port)?;

        // 后台：发现事件 → 设备表 + UI 事件。
        {
            let inner = Arc::clone(&inner);
            tokio::spawn(async move {
                while let Some(event) = disc_rx.recv().await {
                    match event {
                        DiscoveryEvent::Found(dev) => {
                            inner
                                .devices
                                .lock()
                                .unwrap()
                                .insert(dev.uuid.clone(), dev.clone());
                            let _ = inner.events.send(NetEvent::DeviceFound(Device {
                                id: dev.uuid,
                                name: dev.name,
                                online: true,
                            }));
                        }
                        DiscoveryEvent::Lost { uuid } => {
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
            let local = LocalHello {
                uuid: identity.uuid.clone(),
                name: identity.name.clone(),
            };
            tokio::spawn(async move {
                loop {
                    let (stream, _peer) = match listener.accept().await {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let inner = Arc::clone(&inner);
                    let local = local.clone();
                    tokio::spawn(async move {
                        match handshake(stream, &local).await {
                            Ok(session) => {
                                let pairing_id = inner.next_id.fetch_add(1, Ordering::Relaxed);
                                let peer_uuid = session.peer.uuid.clone();
                                let peer_name = session.peer.name.clone();
                                let pin = session.pin.clone();
                                inner.pending.lock().unwrap().insert(pairing_id, session);
                                let _ = inner.events.send(NetEvent::IncomingPairing {
                                    pairing_id,
                                    peer_uuid,
                                    peer_name,
                                    pin,
                                });
                            }
                            Err(e) => {
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

        let service = Self {
            identity,
            inner,
            _discovery: Arc::new(discovery),
        };
        Ok((service, events_rx))
    }

    /// 本机身份。
    pub fn identity(&self) -> &DeviceIdentity {
        &self.identity
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
            match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                TcpStream::connect(addr),
            )
            .await
            {
                Ok(Ok(s)) => {
                    stream = Some(s);
                    break;
                }
                Ok(Err(e)) => last_err = NetworkError::Io(format!("{addr} 连接失败: {e}")),
                Err(_) => last_err = NetworkError::Io(format!("{addr} 连接超时")),
            }
        }
        let stream = stream.ok_or(last_err)?;

        let local = LocalHello {
            uuid: self.identity.uuid.clone(),
            name: self.identity.name.clone(),
        };
        let session = handshake(stream, &local).await?;
        let pin = session.pin.clone();
        let pairing_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
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
        session.send_conversation(conversation).await?;
        // 等待对端 Ack（尽力而为）。
        let _ = session.recv().await;
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
