//! 线缆协议：长度前缀分帧 + 握手/应用消息定义。
//!
//! 每帧 = `u32 大端长度前缀` + `载荷字节`。握手帧载荷为明文 JSON（[`Hello`]）；
//! 配对完成后，应用帧载荷为 AES-256-GCM 密文（解出后再 JSON 解析为 [`AppMessage`]）。

use cs_core::Conversation;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::NetworkError;

/// 单帧最大字节数（16 MiB），防御异常长度导致的过量分配。
const MAX_FRAME_LEN: u32 = 16 * 1024 * 1024;

/// 握手消息：交换公钥与身份。明文传输（公钥本就是公开值）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    /// 协议版本。
    pub version: u8,
    /// 本端设备 UUID。
    pub uuid: String,
    /// 本端显示名。
    pub name: String,
    /// 本端 X25519 公钥（32 字节）。
    pub pubkey: [u8; 32],
}

/// 配对完成后承载于加密帧内的应用消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum AppMessage {
    /// 推送一段对话。
    PushConversation(Conversation),
    /// 接收方确认收妥。
    Ack,
}

/// 写出一帧：长度前缀 + 载荷。
pub async fn write_frame<W>(w: &mut W, payload: &[u8]) -> Result<(), NetworkError>
where
    W: AsyncWriteExt + Unpin,
{
    let len = payload.len();
    if len as u64 > MAX_FRAME_LEN as u64 {
        return Err(NetworkError::Protocol("帧过大".into()));
    }
    w.write_all(&(len as u32).to_be_bytes())
        .await
        .map_err(|e| NetworkError::Io(e.to_string()))?;
    w.write_all(payload)
        .await
        .map_err(|e| NetworkError::Io(e.to_string()))?;
    w.flush()
        .await
        .map_err(|e| NetworkError::Io(e.to_string()))?;
    Ok(())
}

/// 读入一帧载荷。
pub async fn read_frame<R>(r: &mut R) -> Result<Vec<u8>, NetworkError>
where
    R: AsyncReadExt + Unpin,
{
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)
        .await
        .map_err(|e| NetworkError::Io(e.to_string()))?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME_LEN {
        return Err(NetworkError::Protocol("帧过大".into()));
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload)
        .await
        .map_err(|e| NetworkError::Io(e.to_string()))?;
    Ok(payload)
}

/// 写出一个握手帧（JSON 明文）。
pub async fn write_hello<W>(w: &mut W, hello: &Hello) -> Result<(), NetworkError>
where
    W: AsyncWriteExt + Unpin,
{
    let bytes = serde_json::to_vec(hello)?;
    write_frame(w, &bytes).await
}

/// 读入一个握手帧。
pub async fn read_hello<R>(r: &mut R) -> Result<Hello, NetworkError>
where
    R: AsyncReadExt + Unpin,
{
    let bytes = read_frame(r).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frame_roundtrips_over_duplex() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        let payload = b"contextsend".to_vec();
        write_frame(&mut a, &payload).await.unwrap();
        let got = read_frame(&mut b).await.unwrap();
        assert_eq!(got, payload);
    }

    #[tokio::test]
    async fn hello_roundtrips() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        let hello = Hello {
            version: 1,
            uuid: "u-1".into(),
            name: "晨雾微风".into(),
            pubkey: [7u8; 32],
        };
        write_hello(&mut a, &hello).await.unwrap();
        let got = read_hello(&mut b).await.unwrap();
        assert_eq!(got.uuid, "u-1");
        assert_eq!(got.pubkey, [7u8; 32]);
    }
}
