//! Proxy profile / group / lifecycle model (NP-025 schema surface).

#![forbid(unsafe_code)]

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

/// Unified proxy node profile (credentials redacted in summaries).
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
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ProxyProfile {
    /// Safe summary without secrets.
    pub fn redacted_summary(&self) -> String {
        format!(
            "ProxyProfile(id={}, name={}, protocol={:?}, server={}, port={})",
            self.id, self.name, self.protocol, self.server, self.port
        )
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
            tags: vec![],
        };
        let s = p.redacted_summary();
        assert!(!s.contains("secret"));
        assert!(s.contains("example.com"));
    }

    #[test]
    fn lifecycle_names() {
        assert_eq!(ProxyLifecycle::Running.as_str(), "running");
    }
}
