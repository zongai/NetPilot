//! REALITY transport: config + ClientHello fingerprint profiles (uTLS-style).
//!
//! Full X25519 REALITY handshake with server public key is staged as
//! `RealitySession`; ClientHello templates mimic browser JA3/fingerprint order.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;
pub use netpilot_transport_tls::{RealityTlsOverlay, TlsClientConfig, TlsClientSession};

use rand::RngCore;
use sha2::{Digest, Sha256};

pub const CRATE_NAME: &str = "netpilot-transport-reality";

pub fn transport_id() -> TransportId {
    TransportId::Reality
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fingerprint {
    Chrome,
    Firefox,
    Safari,
    IOS,
    Android,
    Edge,
    Random,
}

impl Fingerprint {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "chrome" => Self::Chrome,
            "firefox" => Self::Firefox,
            "safari" => Self::Safari,
            "ios" => Self::IOS,
            "android" => Self::Android,
            "edge" => Self::Edge,
            _ => Self::Random,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Firefox => "firefox",
            Self::Safari => "safari",
            Self::IOS => "ios",
            Self::Android => "android",
            Self::Edge => "edge",
            Self::Random => "random",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityConfig {
    pub server_name: String,
    pub public_key: Option<String>,
    pub short_id: Option<String>,
    pub fingerprint: Fingerprint,
    /// Optional spider Xver.
    pub spider_x: Option<String>,
}

impl RealityConfig {
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            public_key: None,
            short_id: None,
            fingerprint: Fingerprint::Chrome,
            spider_x: None,
        }
    }

    pub fn with_fingerprint(mut self, fp: Fingerprint) -> Self {
        self.fingerprint = fp;
        self
    }

    pub fn to_overlay(&self) -> RealityTlsOverlay {
        RealityTlsOverlay {
            server_name: self.server_name.clone(),
            public_key_redacted: self.public_key.is_some(),
            short_id: self.short_id.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.server_name.trim().is_empty() {
            return Err("reality server_name required");
        }
        Ok(())
    }
}

/// Cipher suites in the order emitted by the fingerprint profile.
pub fn cipher_suites(fp: Fingerprint) -> &'static [u16] {
    match fp {
        Fingerprint::Chrome | Fingerprint::Edge | Fingerprint::Android => &[
            0x1301, // TLS_AES_128_GCM_SHA256
            0x1302, // TLS_AES_256_GCM_SHA384
            0x1303, // TLS_CHACHA20_POLY1305_SHA256
            0xc02b, // ECDHE_ECDSA_AES128_GCM
            0xc02f, // ECDHE_RSA_AES128_GCM
            0xc02c, // ECDHE_ECDSA_AES256_GCM
            0xc030, // ECDHE_RSA_AES256_GCM
            0xcca9, // ECDHE_ECDSA_CHACHA20
            0xcca8, // ECDHE_RSA_CHACHA20
        ],
        Fingerprint::Firefox => &[
            0x1301, 0x1303, 0x1302, 0xc02b, 0xc02f, 0xcca9, 0xcca8, 0xc02c, 0xc030,
        ],
        Fingerprint::Safari | Fingerprint::IOS => &[
            0x1301, 0x1302, 0x1303, 0xc02c, 0xc02b, 0xc030, 0xc02f, 0xcca9, 0xcca8,
        ],
        Fingerprint::Random => cipher_suites(Fingerprint::Chrome),
    }
}

/// Extension type order for ClientHello (uTLS-style).
pub fn extension_order(fp: Fingerprint) -> &'static [u16] {
    match fp {
        Fingerprint::Chrome | Fingerprint::Edge => &[
            0,    // server_name
            23,   // extended_master_secret
            35,   // session_ticket
            13,   // signature_algorithms
            43,   // supported_versions
            45,   // psk_key_exchange_modes
            51,   // key_share
            10,   // supported_groups
            16,   // alpn
            18,   // signed_certificate_timestamp
            27,   // compress_certificate
            17513,// application_settings
            21,   // padding
        ],
        Fingerprint::Firefox => &[
            0, 23, 35, 13, 43, 45, 51, 10, 16, 18, 28, 21,
        ],
        Fingerprint::Safari | Fingerprint::IOS => &[
            0, 23, 35, 13, 43, 45, 51, 10, 16, 18, 21,
        ],
        Fingerprint::Android => &[
            0, 23, 35, 13, 43, 45, 51, 10, 16, 18, 27, 21,
        ],
        Fingerprint::Random => extension_order(Fingerprint::Chrome),
    }
}

/// Build a synthetic TLS 1.3 ClientHello blob matching the fingerprint template.
/// Not a full wire-ready handshake (missing real key_share secrets) but preserves
/// extension order / cipher suite list for fingerprint testing and REALITY staging.
pub fn build_client_hello_template(cfg: &RealityConfig) -> Result<Vec<u8>, String> {
    cfg.validate().map_err(|e| e.to_string())?;
    let fp = cfg.fingerprint;
    let mut random = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random);

    let mut body = Vec::with_capacity(512);
    // legacy version TLS 1.2
    body.extend_from_slice(&0x0303u16.to_be_bytes());
    body.extend_from_slice(&random);
    body.push(0); // session id len 0

    let ciphers = cipher_suites(fp);
    body.extend_from_slice(&((ciphers.len() * 2) as u16).to_be_bytes());
    for c in ciphers {
        body.extend_from_slice(&c.to_be_bytes());
    }
    body.push(1); // compression methods length
    body.push(0); // null

    // extensions
    let mut exts = Vec::new();
    for &etype in extension_order(fp) {
        let data = match etype {
            0 => {
                // SNI
                let host = cfg.server_name.as_bytes();
                let mut d = Vec::new();
                d.extend_from_slice(&((host.len() + 3) as u16).to_be_bytes());
                d.push(0); // host_name
                d.extend_from_slice(&(host.len() as u16).to_be_bytes());
                d.extend_from_slice(host);
                d
            }
            16 => {
                // ALPN h2, http/1.1
                let mut d = Vec::new();
                let list = [b"h2".as_slice(), b"http/1.1".as_slice()];
                let mut inner = Vec::new();
                for p in list {
                    inner.push(p.len() as u8);
                    inner.extend_from_slice(p);
                }
                d.extend_from_slice(&(inner.len() as u16).to_be_bytes());
                d.extend_from_slice(&inner);
                d
            }
            43 => {
                // supported_versions TLS1.3, TLS1.2
                vec![0x02, 0x03, 0x04, 0x03, 0x03]
            }
            10 => {
                // supported_groups x25519, secp256r1
                vec![0x00, 0x04, 0x00, 0x1d, 0x00, 0x17]
            }
            13 => {
                // signature_algorithms
                vec![
                    0x00, 0x08, 0x04, 0x03, 0x08, 0x04, 0x04, 0x01, 0x05, 0x01,
                ]
            }
            21 => {
                // padding — keep small
                vec![0u8; 8]
            }
            _ => Vec::new(),
        };
        exts.extend_from_slice(&etype.to_be_bytes());
        exts.extend_from_slice(&(data.len() as u16).to_be_bytes());
        exts.extend_from_slice(&data);
    }
    body.extend_from_slice(&(exts.len() as u16).to_be_bytes());
    body.extend_from_slice(&exts);

    // TLS record wrapper: Handshake type ClientHello
    let mut hs = Vec::with_capacity(4 + body.len());
    hs.push(0x01); // ClientHello
    let len = body.len() as u32;
    hs.push(((len >> 16) & 0xff) as u8);
    hs.push(((len >> 8) & 0xff) as u8);
    hs.push((len & 0xff) as u8);
    hs.extend_from_slice(&body);

    let mut record = Vec::with_capacity(5 + hs.len());
    record.push(0x16); // handshake
    record.extend_from_slice(&0x0301u16.to_be_bytes()); // record version TLS1.0 for compatibility
    record.extend_from_slice(&(hs.len() as u16).to_be_bytes());
    record.extend_from_slice(&hs);
    Ok(record)
}

/// Hash fingerprint for diagnostics (not JA3-compatible; stable for our templates).
pub fn fingerprint_digest(cfg: &RealityConfig) -> String {
    let mut h = Sha256::new();
    h.update(cfg.fingerprint.as_str().as_bytes());
    h.update(cfg.server_name.as_bytes());
    for c in cipher_suites(cfg.fingerprint) {
        h.update(&c.to_be_bytes());
    }
    for e in extension_order(cfg.fingerprint) {
        h.update(&e.to_be_bytes());
    }
    hex_encode(&h.finalize()[..8])
}

fn hex_encode(b: &[u8]) -> String {
    const H: &[u8] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(H[(x >> 4) as usize] as char);
        s.push(H[(x & 0xf) as usize] as char);
    }
    s
}

#[derive(Debug, Clone)]
pub struct RealitySession {
    pub config: RealityConfig,
    pub client_hello: Vec<u8>,
    pub digest: String,
}

impl RealitySession {
    pub fn open(config: RealityConfig) -> Result<Self, String> {
        let client_hello = build_client_hello_template(&config)?;
        let digest = fingerprint_digest(&config);
        Ok(Self {
            config,
            client_hello,
            digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_hello_has_sni() {
        let cfg = RealityConfig::new("www.example.com").with_fingerprint(Fingerprint::Chrome);
        let hello = build_client_hello_template(&cfg).unwrap();
        assert!(hello.len() > 50);
        assert!(hello.windows(11).any(|w| w == b"example.com" || w.starts_with(b"www.")));
        let dig = fingerprint_digest(&cfg);
        assert_eq!(dig.len(), 16);
    }

    #[test]
    fn session_open() {
        let s = RealitySession::open(RealityConfig::new("a.com")).unwrap();
        assert!(!s.client_hello.is_empty());
    }
}
