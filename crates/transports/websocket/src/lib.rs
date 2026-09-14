//! WebSocket transport surface (path / host headers for VLESS/VMess/Trojan).

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;

pub const CRATE_NAME: &str = "netpilot-transport-websocket";

pub fn transport_id() -> TransportId {
    TransportId::Websocket
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WsClientConfig {
    pub path: String,
    pub host_header: Option<String>,
    pub early_data: bool,
}

impl Default for WsClientConfig {
    fn default() -> Self {
        Self {
            path: "/".into(),
            host_header: None,
            early_data: false,
        }
    }
}

impl WsClientConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.path.is_empty() || !self.path.starts_with('/') {
            return Err("ws path must be non-empty and start with /");
        }
        Ok(())
    }

    pub fn upgrade_request(&self, authority: &str) -> String {
        let host = self.host_header.as_deref().unwrap_or(authority);
        format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
            path = self.path,
            host = host
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Websocket);
    }

    #[test]
    fn upgrade_contains_path() {
        let c = WsClientConfig {
            path: "/vless".into(),
            host_header: Some("example.com".into()),
            early_data: false,
        };
        let r = c.upgrade_request("example.com");
        assert!(r.contains("GET /vless HTTP/1.1"));
        assert!(r.contains("Upgrade: websocket"));
    }
}
