//! Trojan + transport compatibility (NP-120) and dial integration notes.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::{compatibility_matrix, Endpoint, ProtocolId, TransportId};

pub const CRATE_NAME: &str = "netpilot-protocol-trojan";

#[derive(Debug, Clone)]
pub struct TrojanConfig {
    pub endpoint: Endpoint,
    pub password_redacted: bool,
    pub transport: TransportId,
    pub password: Option<String>,
    pub sni: Option<String>,
}

impl TrojanConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            endpoint: Endpoint {
                host: host.into(),
                port,
            },
            password_redacted: true,
            transport: TransportId::Tls,
            password: None,
            sni: None,
        }
    }

    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self.password_redacted = true;
        self
    }

    pub fn protocol_id(&self) -> ProtocolId {
        ProtocolId::Trojan
    }

    pub fn supports_transport(&self, t: TransportId) -> bool {
        compatibility_matrix()
            .iter()
            .any(|e| e.protocol == ProtocolId::Trojan && e.transport == t && e.supported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_ws() {
        let c = TrojanConfig::new("h", 443);
        assert!(c.supports_transport(TransportId::Websocket));
    }
}
