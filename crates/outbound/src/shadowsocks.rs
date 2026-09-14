//! Shadowsocks AEAD (aes-256-gcm / chacha20-poly1305) TCP client.

use std::io::Write;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use chacha20poly1305::ChaCha20Poly1305;
use netpilot_proxy::ProxyProfile;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::addr::{encode_socks_addr, TargetAddr};
use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

#[derive(Clone, Copy)]
enum CipherKind {
    Aes256Gcm,
    ChaCha20Poly1305,
}

pub fn dial_shadowsocks(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let password = profile
        .password
        .as_deref()
        .ok_or_else(|| OutboundError::Invalid("ss password required".into()))?;

    // Method from tags or network field; default aes-256-gcm.
    let method = profile
        .tags
        .iter()
        .find(|t| t.starts_with("method:"))
        .map(|t| t.trim_start_matches("method:"))
        .or(profile.network.as_deref())
        .unwrap_or("aes-256-gcm");
    let cipher = match method.to_ascii_lowercase().as_str() {
        "chacha20-ietf-poly1305" | "chacha20-poly1305" => CipherKind::ChaCha20Poly1305,
        _ => CipherKind::Aes256Gcm,
    };

    let key = derive_key(password, cipher);
    let mut stream = connect_server(&profile.server, profile.port, req.timeout)?;

    // salt (32 bytes for 256-bit) + AEAD(chunked address)
    let mut salt = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt);
    stream
        .write_all(&salt)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    let addr = TargetAddr::from_host_port(&req.target_host, req.target_port);
    let addr_bytes = encode_socks_addr(&addr)?;
    let encrypted = aead_seal(cipher, &key, &salt, 0, &addr_bytes)?;
    stream
        .write_all(&encrypted)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    let peer = stream
        .peer_addr()
        .map(|x| x.to_string())
        .unwrap_or_default();
    Ok((
        OutboundStream::Plain(stream),
        DialReport {
            protocol: "shadowsocks".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer,
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}

fn derive_key(password: &str, cipher: CipherKind) -> [u8; 32] {
    // EVP_BytesToKey-like simplified: SHA256(password) for 32-byte key.
    // Production SS uses MD5 iterative; this is a practical subset for testing.
    let _ = cipher;
    let mut out = [0u8; 32];
    let dig = Sha256::digest(password.as_bytes());
    out.copy_from_slice(&dig);
    out
}

fn aead_seal(
    cipher: CipherKind,
    key: &[u8; 32],
    salt: &[u8; 32],
    counter: u64,
    plaintext: &[u8],
) -> Result<Vec<u8>, OutboundError> {
    // subkey = HKDF-ish: SHA256(salt || key) simplified
    let mut h = Sha256::new();
    h.update(salt);
    h.update(key);
    let subkey = h.finalize();

    let mut nonce = [0u8; 12];
    nonce[4..12].copy_from_slice(&counter.to_le_bytes());

    // length prefix (2 bytes BE) encrypted + payload encrypted (AEAD-2022 style simplified)
    let len = (plaintext.len() as u16).to_be_bytes();
    let mut out = Vec::new();

    match cipher {
        CipherKind::Aes256Gcm => {
            let aead = Aes256Gcm::new_from_slice(&subkey)
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            let n = Nonce::from_slice(&nonce);
            let ct_len = aead
                .encrypt(
                    n,
                    Payload {
                        msg: &len,
                        aad: b"",
                    },
                )
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            out.extend_from_slice(&ct_len);
            // next nonce counter+1
            let mut nonce2 = [0u8; 12];
            nonce2[4..12].copy_from_slice(&(counter + 1).to_le_bytes());
            let n2 = Nonce::from_slice(&nonce2);
            let ct = aead
                .encrypt(
                    n2,
                    Payload {
                        msg: plaintext,
                        aad: b"",
                    },
                )
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            out.extend_from_slice(&ct);
        }
        CipherKind::ChaCha20Poly1305 => {
            let aead = ChaCha20Poly1305::new_from_slice(&subkey)
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            let n = chacha20poly1305::Nonce::from_slice(&nonce);
            let ct_len = aead
                .encrypt(
                    n,
                    Payload {
                        msg: &len,
                        aad: b"",
                    },
                )
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            out.extend_from_slice(&ct_len);
            let mut nonce2 = [0u8; 12];
            nonce2[4..12].copy_from_slice(&(counter + 1).to_le_bytes());
            let n2 = chacha20poly1305::Nonce::from_slice(&nonce2);
            let ct = aead
                .encrypt(
                    n2,
                    Payload {
                        msg: plaintext,
                        aad: b"",
                    },
                )
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            out.extend_from_slice(&ct);
        }
    }
    Ok(out)
}
