//! ShadowsocksR legacy compatibility (NP-116).
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::{Endpoint, ProtocolId};
pub const CRATE_NAME: &str = "netpilot-protocol-shadowsocksr";
#[derive(Debug, Clone)]
pub struct ShadowsocksRConfig {
    pub endpoint: Endpoint,
    pub method: String,
    pub protocol: String,
    pub obfs: String,
}
impl ShadowsocksRConfig {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            endpoint: Endpoint { host: host.into(), port },
            method: "aes-256-cfb".into(),
            protocol: "origin".into(),
            obfs: "plain".into(),
        }
    }
    pub fn protocol_id(&self) -> ProtocolId { ProtocolId::ShadowsocksR }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_defaults() {
        let c = ShadowsocksRConfig::new("x", 1);
        assert_eq!(c.obfs, "plain");
    }
}
