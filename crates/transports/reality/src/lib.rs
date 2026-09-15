//! REALITY transport: config + ClientHello fingerprint profiles (uTLS-style).
//!
//! Full X25519 REALITY handshake with server public key is staged as
//! `RealitySession`; ClientHello templates mimic browser JA3/fingerprint order.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;
pub use netpilot_transport_tls::{RealityTlsOverlay, TlsClientConfig, TlsClientSession};

use hkdf::Hkdf;
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

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
            0,     // server_name
            23,    // extended_master_secret
            35,    // session_ticket
            13,    // signature_algorithms
            43,    // supported_versions
            45,    // psk_key_exchange_modes
            51,    // key_share
            10,    // supported_groups
            16,    // alpn
            18,    // signed_certificate_timestamp
            27,    // compress_certificate
            17513, // application_settings
            21,    // padding
        ],
        Fingerprint::Firefox => &[0, 23, 35, 13, 43, 45, 51, 10, 16, 18, 28, 21],
        Fingerprint::Safari | Fingerprint::IOS => &[0, 23, 35, 13, 43, 45, 51, 10, 16, 18, 21],
        Fingerprint::Android => &[0, 23, 35, 13, 43, 45, 51, 10, 16, 18, 27, 21],
        Fingerprint::Random => extension_order(Fingerprint::Chrome),
    }
}

/// Parse server X25519 public key (32-byte hex or raw base-ish hex).
pub fn parse_x25519_pub(hex_or_b64ish: &str) -> Result<[u8; 32], String> {
    let s = hex_or_b64ish.trim();
    let bytes = if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(s).map_err(|e| e.to_string())?
    } else {
        // try raw hex without strict length
        hex::decode(s).map_err(|e| format!("public_key decode: {e}"))?
    };
    if bytes.len() != 32 {
        return Err(format!("public_key must be 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// REALITY auth material: X25519 ECDH + HKDF → session auth key embedded in SessionID.
#[derive(Debug, Clone)]
pub struct RealityAuthKeys {
    pub client_public: [u8; 32],
    pub shared_secret: [u8; 32],
    pub auth_key: [u8; 16],
    pub short_id: Vec<u8>,
}

pub fn derive_reality_auth(cfg: &RealityConfig) -> Result<RealityAuthKeys, String> {
    let server_pub_bytes = match &cfg.public_key {
        Some(pk) => parse_x25519_pub(pk)?,
        None => return Err("reality public_key required for auth handshake".into()),
    };
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let secret = StaticSecret::from(seed);
    let client_public = PublicKey::from(&secret);
    let server_pub = PublicKey::from(server_pub_bytes);
    let shared = secret.diffie_hellman(&server_pub);
    let mut shared_secret = [0u8; 32];
    shared_secret.copy_from_slice(shared.as_bytes());

    let short_id = match &cfg.short_id {
        Some(s) => {
            let t = s.trim();
            if t.chars().all(|c| c.is_ascii_hexdigit()) && !t.is_empty() {
                hex::decode(t).unwrap_or_default()
            } else {
                t.as_bytes().to_vec()
            }
        }
        None => Vec::new(),
    };

    let hk = Hkdf::<Sha256>::new(Some(b"REALITY"), &shared_secret);
    let mut auth_key = [0u8; 16];
    let mut info = Vec::from(&b"AUTH"[..]);
    info.extend_from_slice(cfg.server_name.as_bytes());
    info.extend_from_slice(&short_id);
    hk.expand(&info, &mut auth_key).map_err(|e| e.to_string())?;

    Ok(RealityAuthKeys {
        client_public: *client_public.as_bytes(),
        shared_secret,
        auth_key,
        short_id,
    })
}

/// Build ClientHello with SessionID carrying REALITY auth + real X25519 key_share.
pub fn build_client_hello_template(cfg: &RealityConfig) -> Result<Vec<u8>, String> {
    build_client_hello_with_auth(cfg, None)
}

pub fn build_client_hello_with_auth(
    cfg: &RealityConfig,
    auth: Option<&RealityAuthKeys>,
) -> Result<Vec<u8>, String> {
    cfg.validate().map_err(|e| e.to_string())?;
    let fp = cfg.fingerprint;
    let mut random = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random);

    // Session ID: 32 bytes — auth_key || short_id pad || random (REALITY-style embedding)
    let mut session_id = [0u8; 32];
    if let Some(a) = auth {
        session_id[..16].copy_from_slice(&a.auth_key);
        let sid = &a.short_id;
        let n = sid.len().min(8);
        session_id[16..16 + n].copy_from_slice(&sid[..n]);
        rand::thread_rng().fill_bytes(&mut session_id[24..]);
    } else {
        rand::thread_rng().fill_bytes(&mut session_id);
    }

    // X25519 key_share public
    let ks_pub = if let Some(a) = auth {
        a.client_public
    } else {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let secret = StaticSecret::from(seed);
        *PublicKey::from(&secret).as_bytes()
    };

    let mut body = Vec::with_capacity(768);
    body.extend_from_slice(&0x0303u16.to_be_bytes());
    body.extend_from_slice(&random);
    body.push(32); // session id length
    body.extend_from_slice(&session_id);

    let ciphers = cipher_suites(fp);
    body.extend_from_slice(&((ciphers.len() * 2) as u16).to_be_bytes());
    for c in ciphers {
        body.extend_from_slice(&c.to_be_bytes());
    }
    body.push(1);
    body.push(0);

    let mut exts = Vec::new();
    for &etype in extension_order(fp) {
        let data = match etype {
            0 => {
                let host = cfg.server_name.as_bytes();
                let mut d = Vec::new();
                d.extend_from_slice(&((host.len() + 3) as u16).to_be_bytes());
                d.push(0);
                d.extend_from_slice(&(host.len() as u16).to_be_bytes());
                d.extend_from_slice(host);
                d
            }
            16 => {
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
            43 => vec![0x02, 0x03, 0x04, 0x03, 0x03],
            10 => vec![0x00, 0x04, 0x00, 0x1d, 0x00, 0x17],
            13 => {
                vec![0x00, 0x08, 0x04, 0x03, 0x08, 0x04, 0x04, 0x01, 0x05, 0x01]
            }
            45 => vec![0x01, 0x01], // psk_key_exchange_modes
            51 => {
                // key_share: x25519
                let mut d = Vec::new();
                d.extend_from_slice(&0x0024u16.to_be_bytes()); // client shares len 36
                d.extend_from_slice(&0x001du16.to_be_bytes()); // group x25519
                d.extend_from_slice(&0x0020u16.to_be_bytes()); // key len 32
                d.extend_from_slice(&ks_pub);
                d
            }
            21 => vec![0u8; 16],
            _ => Vec::new(),
        };
        exts.extend_from_slice(&etype.to_be_bytes());
        exts.extend_from_slice(&(data.len() as u16).to_be_bytes());
        exts.extend_from_slice(&data);
    }
    body.extend_from_slice(&(exts.len() as u16).to_be_bytes());
    body.extend_from_slice(&exts);

    let mut hs = Vec::with_capacity(4 + body.len());
    hs.push(0x01);
    let len = body.len() as u32;
    hs.push(((len >> 16) & 0xff) as u8);
    hs.push(((len >> 8) & 0xff) as u8);
    hs.push((len & 0xff) as u8);
    hs.extend_from_slice(&body);

    let mut record = Vec::with_capacity(5 + hs.len());
    record.push(0x16);
    record.extend_from_slice(&0x0301u16.to_be_bytes());
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
    pub auth: Option<RealityAuthKeys>,
}

impl RealitySession {
    pub fn open(config: RealityConfig) -> Result<Self, String> {
        let auth = if config.public_key.is_some() {
            Some(derive_reality_auth(&config)?)
        } else {
            None
        };
        let client_hello = build_client_hello_with_auth(&config, auth.as_ref())?;
        let digest = fingerprint_digest(&config);
        Ok(Self {
            config,
            client_hello,
            digest,
            auth,
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
        assert!(hello
            .windows(11)
            .any(|w| w == b"example.com" || w.starts_with(b"www.")));
        let dig = fingerprint_digest(&cfg);
        assert_eq!(dig.len(), 16);
    }

    #[test]
    fn session_open() {
        let s = RealitySession::open(RealityConfig::new("a.com")).unwrap();
        assert!(!s.client_hello.is_empty());
    }
}
