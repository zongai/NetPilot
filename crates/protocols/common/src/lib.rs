//! Protocol and transport abstractions (NP-109…NP-112).

#![forbid(unsafe_code)]

use std::collections::HashMap;

pub const CRATE_NAME: &str = "netpilot-protocol-common";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolId {
    Http,
    Https,
    Socks5,
    Shadowsocks,
    Shadowsocks2022,
    ShadowsocksR,
    Vmess,
    Vless,
    Trojan,
}

impl ProtocolId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5 => "socks5",
            Self::Shadowsocks => "shadowsocks",
            Self::Shadowsocks2022 => "shadowsocks2022",
            Self::ShadowsocksR => "shadowsocksr",
            Self::Vmess => "vmess",
            Self::Vless => "vless",
            Self::Trojan => "trojan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportId {
    Tcp,
    Tls,
    Websocket,
    Http2,
    Grpc,
    Reality,
}

impl TransportId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Tls => "tls",
            Self::Websocket => "ws",
            Self::Http2 => "http2",
            Self::Grpc => "grpc",
            Self::Reality => "reality",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub udp: bool,
    pub mux: bool,
    pub tls: bool,
    pub reality: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    /// Redacted in Display; holds protocol-specific secret material.
    pub secret: String,
    pub extra: HashMap<String, String>,
}

impl std::fmt::Display for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Credential(***redacted***)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolUri {
    pub protocol: ProtocolId,
    pub endpoint: Endpoint,
    pub credential: Option<Credential>,
    pub transport: Option<TransportId>,
    pub params: HashMap<String, String>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseUriError {
    UnsupportedScheme(String),
    InvalidFormat,
    MissingHost,
}

impl std::fmt::Display for ParseUriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedScheme(s) => write!(f, "UnsupportedScheme: {s}"),
            Self::InvalidFormat => write!(f, "InvalidFormat"),
            Self::MissingHost => write!(f, "MissingHost"),
        }
    }
}

impl std::error::Error for ParseUriError {}

/// Minimal URI parser for common schemes (not full WHATWG).
pub fn parse_protocol_uri(raw: &str) -> Result<ProtocolUri, ParseUriError> {
    let raw = raw.trim();
    let (scheme, rest) = raw.split_once("://").ok_or(ParseUriError::InvalidFormat)?;
    let protocol = match scheme.to_ascii_lowercase().as_str() {
        "http" => ProtocolId::Http,
        "https" => ProtocolId::Https,
        "socks5" | "socks" => ProtocolId::Socks5,
        "ss" => ProtocolId::Shadowsocks,
        "ssr" => ProtocolId::ShadowsocksR,
        "vmess" => ProtocolId::Vmess,
        "vless" => ProtocolId::Vless,
        "trojan" => ProtocolId::Trojan,
        other => return Err(ParseUriError::UnsupportedScheme(other.into())),
    };
    let (authority, query) = rest.split_once('?').unwrap_or((rest, ""));
    let authority = authority.split('@').next_back().unwrap_or(authority);
    let (host, port) = if let Some((h, p)) = authority.rsplit_once(':') {
        let port: u16 = p.parse().map_err(|_| ParseUriError::InvalidFormat)?;
        (h.to_string(), port)
    } else {
        (authority.to_string(), default_port(protocol))
    };
    if host.is_empty() {
        return Err(ParseUriError::MissingHost);
    }
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(k.to_string(), v.to_string());
    }
    let transport = params
        .get("type")
        .or_else(|| params.get("transport"))
        .and_then(|t| match t.as_str() {
            "ws" | "websocket" => Some(TransportId::Websocket),
            "grpc" => Some(TransportId::Grpc),
            "h2" | "http2" => Some(TransportId::Http2),
            "tcp" => Some(TransportId::Tcp),
            "reality" => Some(TransportId::Reality),
            _ => None,
        });
    Ok(ProtocolUri {
        protocol,
        endpoint: Endpoint { host, port },
        credential: None,
        transport,
        params,
        raw: raw.to_string(),
    })
}

fn default_port(p: ProtocolId) -> u16 {
    match p {
        ProtocolId::Http => 80,
        ProtocolId::Https | ProtocolId::Trojan | ProtocolId::Vless | ProtocolId::Vmess => 443,
        ProtocolId::Socks5 => 1080,
        _ => 443,
    }
}

/// Subscription body: one URI per line.
pub fn parse_subscription_uris(text: &str) -> Vec<Result<ProtocolUri, ParseUriError>> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(parse_protocol_uri)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterState {
    Idle,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Debug, Clone)]
pub struct AdapterHandle {
    pub id: String,
    pub protocol: ProtocolId,
    pub state: AdapterState,
}

#[derive(Debug, Default)]
pub struct AdapterManager {
    adapters: Vec<AdapterHandle>,
}

impl AdapterManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, id: impl Into<String>, protocol: ProtocolId) {
        self.adapters.push(AdapterHandle {
            id: id.into(),
            protocol,
            state: AdapterState::Idle,
        });
    }

    pub fn start(&mut self, id: &str) -> bool {
        if let Some(a) = self.adapters.iter_mut().find(|a| a.id == id) {
            a.state = AdapterState::Running;
            true
        } else {
            false
        }
    }

    pub fn stop(&mut self, id: &str) -> bool {
        if let Some(a) = self.adapters.iter_mut().find(|a| a.id == id) {
            a.state = AdapterState::Stopped;
            true
        } else {
            false
        }
    }

    pub fn list(&self) -> &[AdapterHandle] {
        &self.adapters
    }
}

/// Compatibility matrix entry for protocol × transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatEntry {
    pub protocol: ProtocolId,
    pub transport: TransportId,
    pub supported: bool,
}

pub fn compatibility_matrix() -> Vec<CompatEntry> {
    use ProtocolId::*;
    use TransportId::*;
    let mut out = Vec::new();
    let all_t = [Tcp, Tls, Websocket, Http2, Grpc, Reality];
    for p in [
        Http,
        Https,
        Socks5,
        Shadowsocks,
        Shadowsocks2022,
        ShadowsocksR,
        Vmess,
        Vless,
        Trojan,
    ] {
        for t in all_t {
            let supported = match (p, t) {
                (Http | Https | Socks5, Tcp | Tls) => true,
                (Shadowsocks | Shadowsocks2022 | ShadowsocksR, Tcp) => true,
                (Vmess | Vless | Trojan, Tcp | Tls | Websocket | Http2 | Grpc) => true,
                (Vless, Reality) => true,
                _ => false,
            };
            out.push(CompatEntry {
                protocol: p,
                transport: t,
                supported,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vless() {
        let u = parse_protocol_uri("vless://example.com:443?type=ws").unwrap();
        assert_eq!(u.protocol, ProtocolId::Vless);
        assert_eq!(u.transport, Some(TransportId::Websocket));
    }

    #[test]
    fn matrix_has_vless_reality() {
        assert!(compatibility_matrix()
            .iter()
            .any(|e| e.protocol == ProtocolId::Vless
                && e.transport == TransportId::Reality
                && e.supported));
    }
}
