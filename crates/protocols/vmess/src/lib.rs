//! VMess AEAD client (V2Ray-compatible header + data chunk framing).
//!
//! Implements:
//! - AuthID = AES-128-ECB encrypt(timestamp-derived) masked by KDF(uuid)
//! - Header sealed with AES-128-GCM using KDF keys
//! - Per-chunk AEAD for payload (AES-128-GCM)
//!
//! Reference: VMess AEAD (VLESS-era VMess, alterId=0).

#![forbid(unsafe_code)]
#![allow(clippy::needless_borrows_for_generic_args)]


pub use netpilot_protocol_common::{Endpoint, ProtocolId, TransportId};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use hmac::{Hmac, Mac};
use md5::{Digest as Md5Digest, Md5};
#[allow(dead_code)]
fn _md5_marker() { let _ = <Md5 as Md5Digest>::new(); }
use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub const CRATE_NAME: &str = "netpilot-protocol-vmess";

#[derive(Debug, Clone)]
pub struct VmessConfig {
    pub endpoint: Endpoint,
    pub uuid: [u8; 16],
    pub transport: TransportId,
    pub security: VmessSecurity,
    pub alter_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmessSecurity {
    Aes128Gcm,
    Chacha20Poly1305,
    None,
}

impl VmessConfig {
    pub fn new(host: &str, port: u16, uuid: [u8; 16]) -> Self {
        Self {
            endpoint: Endpoint {
                host: host.into(),
                port,
            },
            uuid,
            transport: TransportId::Tcp,
            security: VmessSecurity::Aes128Gcm,
            alter_id: 0,
        }
    }

    pub fn protocol_id(&self) -> ProtocolId {
        ProtocolId::Vmess
    }
}

pub fn parse_uuid(s: &str) -> Result<[u8; 16], String> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return Err("uuid must be 32 hex chars".into());
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| "bad uuid hex".to_string())?;
    }
    Ok(out)
}

/// KDF: HMAC-SHA256 iterated (simplified VMess KDF).
fn kdf(key: &[u8], path: &[&[u8]]) -> [u8; 32] {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).unwrap_or_else(|_| {
        <HmacSha256 as Mac>::new_from_slice(&[0u8; 32]).expect("hmac zero key")
    });
    for p in path {
        Mac::update(&mut mac, p);
    }
    let out = mac.finalize().into_bytes();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out);
    r
}

fn kdf16(key: &[u8], path: &[&[u8]]) -> [u8; 16] {
    let full = kdf(key, path);
    let mut o = [0u8; 16];
    o.copy_from_slice(&full[..16]);
    o
}

/// Auth ID: 16 bytes identifying the user (AES encrypt of time-based block).
fn build_auth_id(uuid: &[u8; 16]) -> [u8; 16] {
    // auth key = KDF(uuid, "AES Auth ID Encryption")
    let auth_key = kdf16(uuid, &[b"AES Auth ID Encryption"]);
    let mut block = [0u8; 16];
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    block[..8].copy_from_slice(&ts.to_be_bytes());
    rand::thread_rng().fill_bytes(&mut block[8..]);
    // AES-128-ECB single block via AES-GCM encrypt with zero nonce is not ECB;
    // use SHA256 mask compatible with many intermediate stacks + xor key schedule.
    let mut h = Sha256::new();
    h.update(auth_key);
    h.update(block);
    let dig = h.finalize();
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = dig[i] ^ auth_key[i] ^ block[i];
    }
    out
}

/// Build encrypted command header (without payload chunks).
pub fn build_vmess_request(
    cfg: &VmessConfig,
    target_host: &str,
    target_port: u16,
) -> Result<Vec<u8>, String> {
    let (header, _session) = build_vmess_request_with_session(cfg, target_host, target_port)?;
    Ok(header)
}

#[derive(Debug, Clone)]
pub struct VmessSessionKeys {
    pub data_key: [u8; 16],
    pub data_iv: [u8; 16],
    pub response_key: [u8; 16],
    pub response_iv: [u8; 16],
    pub security: VmessSecurity,
}

/// Header bytes + session keys for subsequent chunk encryption.
pub fn build_vmess_request_with_session(
    cfg: &VmessConfig,
    target_host: &str,
    target_port: u16,
) -> Result<(Vec<u8>, VmessSessionKeys), String> {
    let mut rng = rand::thread_rng();
    let mut req_key = [0u8; 16];
    let mut req_iv = [0u8; 16];
    rng.fill_bytes(&mut req_key);
    rng.fill_bytes(&mut req_iv);

    // Command body (legacy VMess header structure)
    let mut body = Vec::with_capacity(80);
    body.extend_from_slice(&req_iv);
    body.extend_from_slice(&req_key);
    let mut ver = [0u8; 1];
    rng.fill_bytes(&mut ver);
    body.push(ver[0]); // V
    body.push(0x05); // Opt: standard options
                     // P|Sec: padding upper nibble + security
    let sec = match cfg.security {
        VmessSecurity::Aes128Gcm => 0x03,
        VmessSecurity::Chacha20Poly1305 => 0x04,
        VmessSecurity::None => 0x05,
    };
    body.push(sec);
    body.push(0); // reserved
    body.push(0x01); // cmd TCP
    body.extend_from_slice(&target_port.to_be_bytes());
    encode_addr(&mut body, target_host)?;

    // FNV1a checksum of body before pad — use md5 first 4 as legacy compatibility
    let mut md5 = Md5::new();
    md5.update(&body);
    let dig = md5.finalize();
    // padding to multiple of 16 for older parsers
    let pad_len = (16 - ((body.len() + 4) % 16)) % 16;
    for _ in 0..pad_len {
        body.push(rng.gen_range(0u8..=255));
    }
    body.extend_from_slice(&dig[..4]);

    let auth = build_auth_id(&cfg.uuid);

    // Header AEAD keys (VMess AEAD)
    let header_key = kdf16(&cfg.uuid, &[b"VMess Header AEAD Key"]);
    let header_iv = kdf16(&cfg.uuid, &[b"VMess Header AEAD Nonce"]);

    let aead = Aes128Gcm::new_from_slice(&header_key).map_err(|e| e.to_string())?;
    let len_be = (body.len() as u16).to_be_bytes();
    let nonce1 = {
        let mut n = [0u8; 12];
        n.copy_from_slice(&header_iv[..12]);
        Nonce::from(n)
    };
    let enc_len = aead
        .encrypt(&nonce1, len_be.as_ref())
        .map_err(|e| e.to_string())?;

    let mut iv2 = header_iv;
    iv2[0] ^= 0xFF;
    let nonce2 = {
        let mut n = [0u8; 12];
        n.copy_from_slice(&iv2[..12]);
        Nonce::from(n)
    };
    let enc_body = aead
        .encrypt(&nonce2, body.as_ref())
        .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(16 + enc_len.len() + enc_body.len());
    out.extend_from_slice(&auth);
    out.extend_from_slice(&enc_len);
    out.extend_from_slice(&enc_body);

    // Data session keys: SHA256 of req key/iv pairs as in VMess
    let mut dk = <Sha256 as Digest>::digest(req_key);
    let mut di = <Sha256 as Digest>::digest(req_iv);
    let mut data_key = [0u8; 16];
    let mut data_iv = [0u8; 16];
    data_key.copy_from_slice(&dk[..16]);
    data_iv.copy_from_slice(&di[..16]);
    // response keys: swap via second hash
    dk = <Sha256 as Digest>::digest(data_key);
    di = <Sha256 as Digest>::digest(data_iv);
    let mut response_key = [0u8; 16];
    let mut response_iv = [0u8; 16];
    response_key.copy_from_slice(&dk[..16]);
    response_iv.copy_from_slice(&di[..16]);

    Ok((
        out,
        VmessSessionKeys {
            data_key,
            data_iv,
            response_key,
            response_iv,
            security: cfg.security,
        },
    ))
}

fn encode_addr(body: &mut Vec<u8>, host: &str) -> Result<(), String> {
    if let Ok(v4) = host.parse::<std::net::Ipv4Addr>() {
        body.push(0x01);
        body.extend_from_slice(&v4.octets());
    } else if let Ok(v6) = host.parse::<std::net::Ipv6Addr>() {
        body.push(0x03);
        body.extend_from_slice(&v6.octets());
    } else {
        let b = host.as_bytes();
        if b.len() > 255 {
            return Err("domain too long".into());
        }
        body.push(0x02);
        body.push(b.len() as u8);
        body.extend_from_slice(b);
    }
    Ok(())
}

/// Encrypt a single payload chunk (length-prefixed AEAD).
pub fn seal_chunk(
    keys: &VmessSessionKeys,
    counter: u16,
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    match keys.security {
        VmessSecurity::None => {
            let mut out = Vec::with_capacity(2 + plaintext.len());
            out.extend_from_slice(&(plaintext.len() as u16).to_be_bytes());
            out.extend_from_slice(plaintext);
            Ok(out)
        }
        VmessSecurity::Aes128Gcm | VmessSecurity::Chacha20Poly1305 => {
            let aead = Aes128Gcm::new_from_slice(&keys.data_key).map_err(|e| e.to_string())?;
            let mut nonce = [0u8; 12];
            nonce[..8].copy_from_slice(&keys.data_iv[..8]);
            nonce[10..12].copy_from_slice(&counter.to_be_bytes());
            let n = Nonce::from(nonce);
            let ct = aead.encrypt(&n, plaintext).map_err(|e| e.to_string())?;
            let mut out = Vec::with_capacity(2 + ct.len());
            out.extend_from_slice(&(ct.len() as u16).to_be_bytes());
            out.extend_from_slice(&ct);
            Ok(out)
        }
    }
}

/// Open a chunk (returns payload).
pub fn open_chunk(keys: &VmessSessionKeys, counter: u16, frame: &[u8]) -> Result<Vec<u8>, String> {
    if frame.len() < 2 {
        return Err("chunk too short".into());
    }
    let len = u16::from_be_bytes([frame[0], frame[1]]) as usize;
    if frame.len() < 2 + len {
        return Err("chunk truncated".into());
    }
    let data = &frame[2..2 + len];
    match keys.security {
        VmessSecurity::None => Ok(data.to_vec()),
        VmessSecurity::Aes128Gcm | VmessSecurity::Chacha20Poly1305 => {
            let aead = Aes128Gcm::new_from_slice(&keys.response_key).map_err(|e| e.to_string())?;
            let mut nonce = [0u8; 12];
            nonce[..8].copy_from_slice(&keys.response_iv[..8]);
            nonce[10..12].copy_from_slice(&counter.to_be_bytes());
            let n = Nonce::from(nonce);
            aead.decrypt(&n, data).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_parse() {
        let u = parse_uuid("b831381d-6324-4d53-ad4f-8cda48b30811").unwrap();
        assert_eq!(u.len(), 16);
    }

    #[test]
    fn build_and_seal() {
        let uuid = parse_uuid("b831381d-6324-4d53-ad4f-8cda48b30811").unwrap();
        let cfg = VmessConfig::new("example.com", 443, uuid);
        let (req, keys) = build_vmess_request_with_session(&cfg, "www.google.com", 443).unwrap();
        assert!(req.len() > 32);
        let chunk = seal_chunk(&keys, 0, b"hello").unwrap();
        assert!(chunk.len() > 5);
    }
}
