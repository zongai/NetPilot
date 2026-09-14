//! VLESS + TLS / REALITY + Vision (NP-118 / NP-119).

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::{Capabilities, Endpoint, ProtocolId, TransportId};

pub const CRATE_NAME: &str = "netpilot-protocol-vless";

#[derive(Debug, Clone)]
pub struct VlessConfig {
    pub endpoint: Endpoint,
    pub flow: Option<String>,
    pub transport: TransportId,
    pub reality: bool,
    pub vision: bool,
}

impl VlessConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            endpoint: Endpoint {
                host: host.into(),
                port,
            },
            flow: None,
            transport: TransportId::Tls,
            reality: false,
            vision: false,
        }
    }

    pub fn with_reality(mut self) -> Self {
        self.reality = true;
        self.transport = TransportId::Reality;
        self
    }

    pub fn with_vision(mut self) -> Self {
        self.vision = true;
        self.flow = Some("xtls-rprx-vision".into());
        self
    }

    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            udp: true,
            mux: false,
            tls: true,
            reality: self.reality,
        }
    }

    pub fn protocol_id(&self) -> ProtocolId {
        ProtocolId::Vless
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reality_vision() {
        let c = VlessConfig::new("h", 443).with_reality().with_vision();
        assert!(c.reality && c.vision);
        assert_eq!(c.transport, TransportId::Reality);
    }
}
