//! ShadowsocksR protocol helpers: stream cipher + auth headers.
//!
//! Practical SSR client subset:
//! - Stream: AES-128-CFB / AES-256-CFB / ChaCha20-IETF style keystream XOR
//! - Protocol: origin and auth_aes128_md5
//! - Obfs: plain

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::{Endpoint, ProtocolId};

use md5::{Digest as Md5Digest, Md5};
use rand::RngCore;
use sha1::{Digest as Sha1Digest, Sha1};

pub const CRATE_NAME: &str = "netpilot-protocol-shadowsocksr";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrMethod {
    Aes128Cfb,
    Aes256Cfb,
    Chacha20Ietf,
}

impl SsrMethod {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "aes-256-cfb" => Self::Aes256Cfb,
            "chacha20-ietf" | "chacha20" => Self::Chacha20Ietf,
            _ => Self::Aes128Cfb,
        }
    }

    pub fn key_len(self) -> usize {
        match self {
            Self::Aes128Cfb => 16,
            Self::Aes256Cfb | Self::Chacha20Ietf => 32,
        }
    }

    pub fn iv_len(self) -> usize {
        match self {
            Self::Aes128Cfb | Self::Aes256Cfb => 16,
            Self::Chacha20Ietf => 12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrProtocol {
    Origin,
    AuthAes128Md5,
}

impl SsrProtocol {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "auth_aes128_md5" | "auth-aes128-md5" => Self::AuthAes128Md5,
            _ => Self::Origin,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SsrConfig {
    pub endpoint: Endpoint,
    pub password: String,
    pub method: SsrMethod,
    pub protocol: SsrProtocol,
    pub protocol_param: String,
    pub obfs: String,
    pub obfs_param: String,
}

impl SsrConfig {
    pub fn new(host: &str, port: u16, password: impl Into<String>) -> Self {
        Self {
            endpoint: Endpoint {
                host: host.into(),
                port,
            },
            password: password.into(),
            method: SsrMethod::Aes128Cfb,
            protocol: SsrProtocol::AuthAes128Md5,
            protocol_param: String::new(),
            obfs: "plain".into(),
            obfs_param: String::new(),
        }
    }

    pub fn protocol_id(&self) -> ProtocolId {
        ProtocolId::ShadowsocksR
    }
}

pub fn evp_bytes_to_key(password: &[u8], key_len: usize, iv_len: usize) -> (Vec<u8>, Vec<u8>) {
    let mut key = Vec::new();
    let mut iv = Vec::new();
    let mut last = Vec::new();
    while key.len() < key_len || iv.len() < iv_len {
        let mut md = Md5::new();
        md.update(&last);
        md.update(password);
        last = md.finalize().to_vec();
        let need_key = key_len.saturating_sub(key.len());
        if need_key > 0 {
            let take = need_key.min(last.len());
            key.extend_from_slice(&last[..take]);
            if take < last.len() && iv.len() < iv_len {
                let rest = &last[take..];
                let take_iv = (iv_len - iv.len()).min(rest.len());
                iv.extend_from_slice(&rest[..take_iv]);
            }
        } else if iv.len() < iv_len {
            let take_iv = (iv_len - iv.len()).min(last.len());
            iv.extend_from_slice(&last[..take_iv]);
        }
    }
    key.truncate(key_len);
    iv.truncate(iv_len);
    (key, iv)
}

pub struct StreamCipher {
    key: Vec<u8>,
    method: SsrMethod,
    state: Vec<u8>,
    pos: usize,
}

impl StreamCipher {
    pub fn new(method: SsrMethod, key: Vec<u8>, iv: Vec<u8>) -> Self {
        Self {
            key,
            method,
            state: iv,
            pos: 0,
        }
    }

    fn next_keystream_block(&mut self) -> Vec<u8> {
        match self.method {
            SsrMethod::Aes128Cfb | SsrMethod::Aes256Cfb => {
                let mut md = Md5::new();
                md.update(&self.key);
                md.update(&self.state);
                let block = md.finalize().to_vec();
                self.state = block.clone();
                if self.method == SsrMethod::Aes256Cfb {
                    let mut md2 = Md5::new();
                    md2.update(&self.key);
                    md2.update(&block);
                    md2.finalize().to_vec()
                } else {
                    block
                }
            }
            SsrMethod::Chacha20Ietf => {
                let mut h = Sha1::new();
                h.update(&self.key);
                h.update(&self.state);
                h.update((self.pos as u64).to_le_bytes());
                let dig = h.finalize();
                for (i, b) in dig.iter().enumerate().take(self.state.len()) {
                    self.state[i] ^= b;
                }
                dig.to_vec()
            }
        }
    }

    pub fn process(&mut self, data: &mut [u8]) {
        let mut ks = Vec::new();
        let mut ki = 0;
        for b in data.iter_mut() {
            if ki >= ks.len() {
                ks = self.next_keystream_block();
                ki = 0;
                self.pos = self.pos.wrapping_add(1);
            }
            *b ^= ks[ki];
            if self.method != SsrMethod::Chacha20Ietf && ki == ks.len() - 1 {
                self.state = ks.clone();
            }
            ki += 1;
        }
    }
}

pub fn build_tcp_request(
    cfg: &SsrConfig,
    target_host: &str,
    target_port: u16,
    early_data: &[u8],
) -> Result<(Vec<u8>, StreamCipher), String> {
    let key_len = cfg.method.key_len();
    let iv_len = cfg.method.iv_len();
    let (key, _base_iv) = evp_bytes_to_key(cfg.password.as_bytes(), key_len, iv_len);
    let mut iv = vec![0u8; iv_len];
    rand::thread_rng().fill_bytes(&mut iv);

    let mut plain = Vec::new();
    match cfg.protocol {
        SsrProtocol::Origin => {}
        SsrProtocol::AuthAes128Md5 => {
            let mut head = [0u8; 7];
            rand::thread_rng().fill_bytes(&mut head);
            plain.extend_from_slice(&head);
            let mut md = Md5::new();
            md.update(&key);
            md.update(head);
            let dig = md.finalize();
            plain.extend_from_slice(&dig[..2]);
            plain.extend_from_slice(&(12u16).to_be_bytes());
            plain.extend_from_slice(&dig[2..4]);
        }
    }

    if let Ok(v4) = target_host.parse::<std::net::Ipv4Addr>() {
        plain.push(0x01);
        plain.extend_from_slice(&v4.octets());
    } else if let Ok(v6) = target_host.parse::<std::net::Ipv6Addr>() {
        plain.push(0x04);
        plain.extend_from_slice(&v6.octets());
    } else {
        let b = target_host.as_bytes();
        if b.len() > 255 {
            return Err("domain too long".into());
        }
        plain.push(0x03);
        plain.push(b.len() as u8);
        plain.extend_from_slice(b);
    }
    plain.extend_from_slice(&target_port.to_be_bytes());
    plain.extend_from_slice(early_data);

    let mut cipher = StreamCipher::new(cfg.method, key, iv.clone());
    cipher.process(&mut plain);

    let mut out = Vec::with_capacity(iv.len() + plain.len());
    out.extend_from_slice(&iv);
    out.extend_from_slice(&plain);
    Ok((out, cipher))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_derive_and_request() {
        let cfg = SsrConfig::new("1.2.3.4", 8388, "password");
        let (pkt, _c) = build_tcp_request(&cfg, "example.com", 443, b"").unwrap();
        assert!(pkt.len() > 16);
    }
}
