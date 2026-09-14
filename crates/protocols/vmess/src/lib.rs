//! VMess adapter (NP-117).
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::{Endpoint, ProtocolId, TransportId};
pub const CRATE_NAME: &str = "netpilot-protocol-vmess";
#[derive(Debug, Clone)]
pub struct VmessConfig {
    pub endpoint: Endpoint,
    pub uuid_redacted: bool,
    pub alter_id: u16,
    pub security: String,
    pub transport: TransportId,
}
impl VmessConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            endpoint: Endpoint { host: host.into(), port },
            uuid_redacted: true,
            alter_id: 0,
            security: "auto".into(),
            transport: TransportId::Tcp,
        }
    }
    pub fn protocol_id(&self) -> ProtocolId { ProtocolId::Vmess }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_tcp() {
        assert_eq!(VmessConfig::new("h", 443).transport, TransportId::Tcp);
    }
}
