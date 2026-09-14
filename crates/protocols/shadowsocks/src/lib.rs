//! Shadowsocks / SS2022 adapters (NP-114 / NP-115).
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::{Capabilities, Endpoint, ProtocolId};
pub const CRATE_NAME: &str = "netpilot-protocol-shadowsocks";
#[derive(Debug, Clone)]
pub struct ShadowsocksConfig {
    pub endpoint: Endpoint,
    pub method: String,
    pub password_redacted: bool,
    pub is_2022: bool,
}
impl ShadowsocksConfig {
    pub fn classic(host: &str, port: u16, method: &str) -> Self {
        Self {
            endpoint: Endpoint { host: host.into(), port },
            method: method.into(),
            password_redacted: true,
            is_2022: false,
        }
    }
    pub fn ss2022(host: &str, port: u16, method: &str) -> Self {
        let mut c = Self::classic(host, port, method);
        c.is_2022 = true;
        c
    }
    pub fn capabilities(&self) -> Capabilities {
        Capabilities { udp: true, mux: false, tls: false, reality: false }
    }
    pub fn protocol_id(&self) -> ProtocolId {
        if self.is_2022 { ProtocolId::Shadowsocks2022 } else { ProtocolId::Shadowsocks }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ss2022_flag() {
        let c = ShadowsocksConfig::ss2022("1.1.1.1", 8388, "2022-blake3-aes-128-gcm");
        assert!(c.is_2022);
        assert!(c.capabilities().udp);
    }
}
