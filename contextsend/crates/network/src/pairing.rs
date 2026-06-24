//! 配对握手与加密会话。
//!
//! 握手是对称的：双方各自生成 X25519 密钥，互发 [`Hello`]，用对端公钥完成 ECDH，
//! 派生出同一会话密钥与同一 6 位配对码。之后所有应用消息走 AES-256-GCM 加密帧。

use cs_core::Conversation;
use tokio::io::{split, AsyncRead, AsyncWrite, ReadHalf, WriteHalf};

use crate::crypto::{KeyExchange, SessionCipher};
use crate::wire::{self, AppMessage, Hello};
use crate::NetworkError;

/// 握手时携带的本端身份。
#[derive(Debug, Clone)]
pub struct LocalHello {
    pub uuid: String,
    pub name: String,
}

/// 握手后获知的对端身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    pub uuid: String,
    pub name: String,
}

/// 一条已建立加密的会话。
pub struct Session<S> {
    reader: ReadHalf<S>,
    writer: WriteHalf<S>,
    cipher: SessionCipher,
    /// 对端身份。
    pub peer: PeerInfo,
    /// 6 位配对码（双方应一致，供用户比对）。
    pub pin: String,
}

/// 在已连接的流上执行配对握手。
///
/// 适用于主动方与被动方——逻辑完全对称。
pub async fn handshake<S>(stream: S, local: &LocalHello) -> Result<Session<S>, NetworkError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let kx = KeyExchange::generate();
    let (mut reader, mut writer) = split(stream);

    let hello = Hello {
        version: 1,
        uuid: local.uuid.clone(),
        name: local.name.clone(),
        pubkey: kx.public_bytes(),
    };

    // 同时收发，避免双方都先 read 造成死锁。
    let (send_res, recv_res) = tokio::join!(
        wire::write_hello(&mut writer, &hello),
        wire::read_hello(&mut reader)
    );
    send_res?;
    let peer_hello = recv_res?;

    if peer_hello.version != 1 {
        return Err(NetworkError::Protocol(format!(
            "不支持的协议版本: {}",
            peer_hello.version
        )));
    }

    let keys = kx.complete(peer_hello.pubkey);

    Ok(Session {
        reader,
        writer,
        cipher: keys.cipher(),
        peer: PeerInfo {
            uuid: peer_hello.uuid,
            name: peer_hello.name,
        },
        pin: keys.pin().to_string(),
    })
}

impl<S> Session<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// 发送一条应用消息（加密）。
    pub async fn send(&mut self, msg: &AppMessage) -> Result<(), NetworkError> {
        let plain = serde_json::to_vec(msg)?;
        let ciphertext = self.cipher.encrypt(&plain)?;
        wire::write_frame(&mut self.writer, &ciphertext).await
    }

    /// 接收一条应用消息（解密）。
    pub async fn recv(&mut self) -> Result<AppMessage, NetworkError> {
        let ciphertext = wire::read_frame(&mut self.reader).await?;
        let plain = self.cipher.decrypt(&ciphertext)?;
        Ok(serde_json::from_slice(&plain)?)
    }

    /// 便捷方法：推送一段对话。
    pub async fn send_conversation(&mut self, convo: &Conversation) -> Result<(), NetworkError> {
        self.send(&AppMessage::PushConversation(convo.clone()))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::ChatMessage;

    fn sample_conversation() -> Conversation {
        Conversation {
            title: Some("跨设备演示".into()),
            model: Some("gpt-4o".into()),
            messages: vec![
                ChatMessage::system("你是一个有用的助手"),
                ChatMessage::user("帮我写个排序"),
                ChatMessage::assistant("好的，这是快排……"),
                ChatMessage::user("再解释一下复杂度"),
                ChatMessage::assistant("平均 O(n log n)"),
            ],
        }
    }

    /// 端到端：本地 TCP 上完成握手 + 推送 5 条消息，验证 PIN 一致且逐字相同。
    #[tokio::test]
    async fn end_to_end_pairing_and_push_over_tcp() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let expected = sample_conversation();
        let expected_for_server = expected.clone();

        // 被动方（接收）。
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let local = LocalHello {
                uuid: "server-uuid".into(),
                name: "服务端".into(),
            };
            let mut session = handshake(stream, &local).await.unwrap();
            let msg = session.recv().await.unwrap();
            session.send(&AppMessage::Ack).await.unwrap();
            (session.pin.clone(), session.peer.clone(), msg)
        });

        // 主动方（发送）。
        let stream = TcpStream::connect(addr).await.unwrap();
        let local = LocalHello {
            uuid: "client-uuid".into(),
            name: "客户端".into(),
        };
        let mut client = handshake(stream, &local).await.unwrap();
        client.send_conversation(&expected).await.unwrap();
        let ack = client.recv().await.unwrap();

        let (server_pin, server_saw_peer, received) = server.await.unwrap();

        // PIN 双方一致。
        assert_eq!(client.pin, server_pin);
        // 身份正确互认。
        assert_eq!(client.peer.uuid, "server-uuid");
        assert_eq!(server_saw_peer.uuid, "client-uuid");
        // 收到 Ack。
        assert!(matches!(ack, AppMessage::Ack));
        // 内容逐字一致。
        match received {
            AppMessage::PushConversation(got) => assert_eq!(got, expected_for_server),
            other => panic!("期望 PushConversation，得到 {other:?}"),
        }
    }
}
