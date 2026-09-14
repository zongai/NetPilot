//! REALITY transport surface — builds on TLS client config.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;
pub use netpilot_transport_tls::{
    RealityTlsOverlay, TlsClientConfig, TlsClientSession, TlsError, TlsSessionState,
};

pub const CRATE_NAME: &str = "netpilot-transport-reality";

pub fn transport_id() -> TransportId {
    TransportId::Reality
}

#[derive(Debug, Clone)]
pub struct RealityConfig {
    pub overlay: RealityTlsOverlay,
}

impl RealityConfig {
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            overlay: RealityTlsOverlay {
                server_name: server_name.into(),
                public_key_redacted: true,
                short_id: None,
            },
        }
    }

    pub fn open_mock_session(&self) -> Result<TlsClientSession, TlsError> {
        let mut session = TlsClientSession::new(self.overlay.to_tls_config())?;
        session.connect()?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Reality);
    }

    #[test]
    fn mock_session() {
        let s = RealityConfig::new("www.example.com")
            .open_mock_session()
            .unwrap();
        assert_eq!(s.state(), TlsSessionState::Connected);
    }
}
