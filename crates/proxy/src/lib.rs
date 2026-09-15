//! Unified proxy model: profile, groups, health, lifecycle, secrets (NP-025…NP-035).

#![forbid(unsafe_code)]

mod endpoint;
mod groups;
mod health;
mod lifecycle;
mod secrets;

pub use endpoint::{
    validate_host, validate_port, validate_profile_endpoint, validate_uuid, EndpointError,
};
pub use groups::{apply_selection, select_member, GroupError, MemberHealth};
pub use health::{HealthRecord, HealthState, HealthTable, ProbePolicy};
pub use lifecycle::{LifecycleError, LifecycleManager};
pub use secrets::{leaks_secret, redact_profile, redacted_field_map, REDACTED, SECRET_FIELD_NAMES};

use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "netpilot-proxy";

/// Supported protocol identifiers in config (wire adapters land in S9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolKind {
    Http,
    Socks5,
    Shadowsocks,
    Shadowsocks2022,
    ShadowsocksR,
    Vmess,
    Vless,
    Trojan,
}

impl ProtocolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Socks5 => "socks5",
            Self::Shadowsocks => "shadowsocks",
            Self::Shadowsocks2022 => "shadowsocks2022",
            Self::ShadowsocksR => "shadowsocksr",
            Self::Vmess => "vmess",
            Self::Vless => "vless",
            Self::Trojan => "trojan",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "http" | "https" => Some(Self::Http),
            "socks5" | "socks" => Some(Self::Socks5),
            "shadowsocks" | "ss" => Some(Self::Shadowsocks),
            "shadowsocks2022" | "ss2022" => Some(Self::Shadowsocks2022),
            "shadowsocksr" | "ssr" => Some(Self::ShadowsocksR),
            "vmess" => Some(Self::Vmess),
            "vless" => Some(Self::Vless),
            "trojan" => Some(Self::Trojan),
            _ => None,
        }
    }
}

/// Transport layer identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Tcp,
    Udp,
    Tls,
    Websocket,
    Http2,
    Grpc,
    Reality,
}

impl TransportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Tls => "tls",
            Self::Websocket => "websocket",
            Self::Http2 => "http2",
            Self::Grpc => "grpc",
            Self::Reality => "reality",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "tcp" => Some(Self::Tcp),
            "udp" => Some(Self::Udp),
            "tls" => Some(Self::Tls),
            "ws" | "websocket" => Some(Self::Websocket),
            "h2" | "http2" => Some(Self::Http2),
            "grpc" => Some(Self::Grpc),
            "reality" => Some(Self::Reality),
            _ => None,
        }
    }
}

/// Unified proxy node profile (NP-030). Secrets must not appear in logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyProfile {
    pub id: String,
    pub name: String,
    pub protocol: ProtocolKind,
    #[serde(default)]
    pub transport: Option<TransportKind>,
    pub server: String,
    pub port: u16,
    /// Never log this field in plain form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// Cipher / security method (ss, ssr, vmess).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cipher: Option<String>,
    /// REALITY server public key (hex).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// REALITY short id (hex).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
    /// uTLS / REALITY fingerprint name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ProxyProfile {
    /// Safe summary without secrets.
    pub fn redacted_summary(&self) -> String {
        format!(
            "ProxyProfile(id={}, name={}, protocol={}, server={}, port={})",
            self.id,
            self.name,
            self.protocol.as_str(),
            self.server,
            self.port
        )
    }

    /// Whether this profile requires UUID-style identity.
    pub fn requires_uuid(&self) -> bool {
        matches!(self.protocol, ProtocolKind::Vmess | ProtocolKind::Vless)
    }

    /// Whether password/credential is expected for the protocol.
    pub fn expects_password(&self) -> bool {
        matches!(
            self.protocol,
            ProtocolKind::Http
                | ProtocolKind::Socks5
                | ProtocolKind::Shadowsocks
                | ProtocolKind::Shadowsocks2022
                | ProtocolKind::ShadowsocksR
                | ProtocolKind::Trojan
        )
    }

    /// Basic field cleanup used by config normalization.
    pub fn normalize_fields(&mut self) {
        self.id = self.id.trim().to_string();
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            self.name = self.id.clone();
        }
        self.server = self.server.trim().to_string();
        for tag in &mut self.tags {
            *tag = tag.trim().to_string();
        }
        self.tags.retain(|t| !t.is_empty());
        self.tags.sort();
        self.tags.dedup();
    }
}

/// Selection policy for a group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupSelect {
    Manual,
    UrlTest,
    Fallback,
}

/// Proxy group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub id: String,
    pub name: String,
    pub select: GroupSelect,
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,
}

impl ProxyGroup {
    pub fn normalize_fields(&mut self) {
        self.id = self.id.trim().to_string();
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            self.name = self.id.clone();
        }
        for m in &mut self.members {
            *m = m.trim().to_string();
        }
        self.members.retain(|m| !m.is_empty());
        if let Some(sel) = self.selected.as_mut() {
            *sel = sel.trim().to_string();
            if sel.is_empty() {
                self.selected = None;
            }
        }
    }
}

/// Lifecycle state for a running proxy adapter instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyLifecycle {
    Created,
    Validating,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

impl ProxyLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Validating => "validating",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_redacts_in_summary() {
        let p = ProxyProfile {
            id: "n1".into(),
            name: "node".into(),
            protocol: ProtocolKind::Vless,
            transport: Some(TransportKind::Reality),
            server: "example.com".into(),
            port: 443,
            password: Some("secret".into()),
            uuid: Some("00000000-0000-0000-0000-000000000000".into()),
            username: None,
            sni: Some("example.com".into()),
            alpn: None,
            path: None,
            host: None,
            flow: None,
            network: None,
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        };
        let s = p.redacted_summary();
        assert!(!s.contains("secret"));
        assert!(s.contains("example.com"));
        assert!(p.requires_uuid());
    }

    #[test]
    fn protocol_parse() {
        assert_eq!(ProtocolKind::parse("ss"), Some(ProtocolKind::Shadowsocks));
        assert_eq!(TransportKind::parse("ws"), Some(TransportKind::Websocket));
    }

    #[test]
    fn lifecycle_names() {
        assert_eq!(ProxyLifecycle::Running.as_str(), "running");
    }
}
