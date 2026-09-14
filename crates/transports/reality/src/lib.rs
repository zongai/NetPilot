//! REALITY transport overlay (NP-119). Full uTLS fingerprint is deferred;
//! config + TLS overlay surface is complete for dial path selection.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;
pub use netpilot_transport_tls::{RealityTlsOverlay, TlsClientConfig, TlsClientSession};

pub const CRATE_NAME: &str = "netpilot-transport-reality";

pub fn transport_id() -> TransportId {
    TransportId::Reality
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityConfig {
    pub server_name: String,
    pub public_key: Option<String>,
    pub short_id: Option<String>,
    pub fingerprint: Option<String>,
}

impl RealityConfig {
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            public_key: None,
            short_id: None,
            fingerprint: None,
        }
    }

    pub fn to_overlay(&self) -> RealityTlsOverlay {
        RealityTlsOverlay {
            server_name: self.server_name.clone(),
            public_key_redacted: self.public_key.is_some(),
            short_id: self.short_id.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.server_name.trim().is_empty() {
            return Err("reality server_name required");
        }
        Ok(())
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
    fn overlay_redacts_key() {
        let c = RealityConfig {
            server_name: "www.example.com".into(),
            public_key: Some("secret".into()),
            short_id: Some("abcd".into()),
            fingerprint: Some("chrome".into()),
        };
        let o = c.to_overlay();
        assert!(o.public_key_redacted);
        assert_eq!(o.short_id.as_deref(), Some("abcd"));
    }
}
