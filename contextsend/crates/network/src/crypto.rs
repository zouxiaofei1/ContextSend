//! 配对密码学：X25519 ECDH 密钥交换、HKDF 派生会话密钥与 6 位配对码（SAS）、
//! 以及 AES-256-GCM 加解密。
//!
//! 配对采用 **SAS（Short Authentication String）** 模型：双方各自做 ECDH 得到相同的
//! 共享密钥，再由共享密钥派生出同一个 6 位数字码，分别显示给用户。用户确认两端码一致，
//! 即可排除中间人（攻击者无法让两条独立 ECDH 产生相同的码）。确认后用同一密钥做 AES-256-GCM。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::NetworkError;

/// HKDF 派生会话密钥所用的 info 串。
const INFO_SESSION: &[u8] = b"contextsend/v1/session-key";
/// HKDF 派生配对码所用的 info 串。
const INFO_SAS: &[u8] = b"contextsend/v1/sas-code";

/// 本端在一次配对中的临时密钥材料。
pub struct KeyExchange {
    secret: StaticSecret,
    public: PublicKey,
}

impl KeyExchange {
    /// 生成一对新的 X25519 密钥。
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// 本端公钥（32 字节），需发送给对端。
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    /// 用对端公钥完成 ECDH，派生出 [`PairedKeys`]（会话密钥 + 6 位配对码）。
    pub fn complete(self, peer_public: [u8; 32]) -> PairedKeys {
        let peer = PublicKey::from(peer_public);
        let shared = self.secret.diffie_hellman(&peer);

        // salt 取双方公钥按字节序排序后拼接，保证两端派生输入完全一致（与谁先发无关）。
        let mut a = self.public.to_bytes();
        let mut b = peer_public;
        if a.as_slice() > b.as_slice() {
            std::mem::swap(&mut a, &mut b);
        }
        let mut salt = Vec::with_capacity(64);
        salt.extend_from_slice(&a);
        salt.extend_from_slice(&b);

        let hk = Hkdf::<Sha256>::new(Some(&salt), shared.as_bytes());

        let mut session_key = [0u8; 32];
        hk.expand(INFO_SESSION, &mut session_key)
            .expect("32 字节在 HKDF 输出范围内");

        let mut sas_bytes = [0u8; 4];
        hk.expand(INFO_SAS, &mut sas_bytes)
            .expect("4 字节在 HKDF 输出范围内");
        // 取 4 字节大端 mod 1_000_000，得到 0..=999999，零填充为 6 位。
        let sas_num = u32::from_be_bytes(sas_bytes) % 1_000_000;
        let pin = format!("{sas_num:06}");

        PairedKeys { session_key, pin }
    }
}

/// ECDH 完成后的成果：会话密钥与展示给用户的 6 位配对码。
#[derive(Clone)]
pub struct PairedKeys {
    session_key: [u8; 32],
    pin: String,
}

impl PairedKeys {
    /// 6 位配对码（双方应一致）。
    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// 构造可加解密的会话密码器。
    pub fn cipher(&self) -> SessionCipher {
        SessionCipher {
            cipher: Aes256Gcm::new(&self.session_key.into()),
        }
    }
}

/// 基于会话密钥的 AES-256-GCM 密码器。
///
/// 每次加密随机生成 12 字节 nonce，并前置到密文中（`nonce || ciphertext`）。
pub struct SessionCipher {
    cipher: Aes256Gcm,
}

impl SessionCipher {
    /// 加密：输出 `nonce(12) || ciphertext(含 16 字节 tag)`。
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| NetworkError::Crypto("加密失败".into()))?;
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// 解密 [`Self::encrypt`] 的输出。
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, NetworkError> {
        if data.len() < 12 {
            return Err(NetworkError::Crypto("密文过短".into()));
        }
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| NetworkError::Crypto("解密失败（密钥不符或数据被篡改）".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_derive_same_key_and_pin() {
        let alice = KeyExchange::generate();
        let bob = KeyExchange::generate();
        let a_pub = alice.public_bytes();
        let b_pub = bob.public_bytes();

        let a_keys = alice.complete(b_pub);
        let b_keys = bob.complete(a_pub);

        assert_eq!(a_keys.pin(), b_keys.pin());
        assert_eq!(a_keys.pin().len(), 6);
        assert!(a_keys.pin().chars().all(|c| c.is_ascii_digit()));

        // 同一会话密钥可互相加解密。
        let ct = a_keys.cipher().encrypt(b"hello").unwrap();
        let pt = b_keys.cipher().decrypt(&ct).unwrap();
        assert_eq!(pt, b"hello");
    }

    #[test]
    fn mitm_produces_mismatched_pin() {
        // 攻击者 mallory 分别与 alice、bob 交换，无法让两端 PIN 相同。
        let alice = KeyExchange::generate();
        let bob = KeyExchange::generate();
        let mallory_a = KeyExchange::generate();
        let mallory_b = KeyExchange::generate();

        let a_keys = alice.complete(mallory_a.public_bytes());
        let b_keys = bob.complete(mallory_b.public_bytes());

        assert_ne!(a_keys.pin(), b_keys.pin());
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let alice = KeyExchange::generate();
        let bob = KeyExchange::generate();
        let keys = alice.complete(bob.public_bytes());

        let cipher = keys.cipher();
        let mut ct = cipher.encrypt(b"secret").unwrap();
        let last = ct.len() - 1;
        ct[last] ^= 0xff;
        assert!(cipher.decrypt(&ct).is_err());
    }
}
