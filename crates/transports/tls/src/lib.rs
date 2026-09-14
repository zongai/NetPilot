//! TLS transport wiring (formal build).
//!
//! Configures SNI, ALPN, and certificate policy. Real rustls / schannel
//! sessions are feature-gated; default CI exercises the configuration path only.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;

pub const CRATE_NAME: &str = "netpilot-transport-tls";

pub fn transport_id() -> TransportId {
    TransportId::Tls
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsBackend {
    /// Platform default (SChannel on Windows when enabled).
    Platform,
    /// rustls (feature `tls-rustls`).
    Rustls,
    /// Config-only / test backend.
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertVerifyMode {
    SystemRoots,
    CustomRoots,
    InsecureSkipVerify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsClientConfig {
    pub server_name: String,
    pub alpn: Vec<String>,
    pub verify: CertVerifyMode,
    pub backend: TlsBackend,
    pub min_version: TlsVersion,
}

impl Default for TlsClientConfig {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            alpn: vec!["h2".into(), "http/1.1".into()],
            verify: CertVerifyMode::SystemRoots,
            backend: TlsBackend::Mock,
            min_version: TlsVersion::Tls12,
        }
    }
}

impl TlsClientConfig {
    pub fn for_host(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            ..Self::default()
        }
    }

    pub fn with_alpn(mut self, protocols: &[&str]) -> Self {
        self.alpn = protocols.iter().map(|s| (*s).to_string()).collect();
        self
    }

    pub fn validate(&self) -> Result<(), TlsError> {
        if self.server_name.trim().is_empty() {
            return Err(TlsError::InvalidConfig("server_name (SNI) required"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsError {
    InvalidConfig(&'static str),
    Handshake(&'static str),
    Unsupported(&'static str),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(m) => write!(f, "InvalidConfig: {m}"),
            Self::Handshake(m) => write!(f, "Handshake: {m}"),
            Self::Unsupported(m) => write!(f, "Unsupported: {m}"),
        }
    }
}

impl std::error::Error for TlsError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsSessionState {
    Idle,
    Connecting,
    Connected,
    Closed,
    Failed,
}

/// Logical TLS client session (mock unless native feature enabled).
#[derive(Debug)]
pub struct TlsClientSession {
    config: TlsClientConfig,
    state: TlsSessionState,
    negotiated_alpn: Option<String>,
}

impl TlsClientSession {
    pub fn new(config: TlsClientConfig) -> Result<Self, TlsError> {
        config.validate()?;
        Ok(Self {
            config,
            state: TlsSessionState::Idle,
            negotiated_alpn: None,
        })
    }

    pub fn state(&self) -> TlsSessionState {
        self.state
    }

    pub fn server_name(&self) -> &str {
        &self.config.server_name
    }

    pub fn negotiated_alpn(&self) -> Option<&str> {
        self.negotiated_alpn.as_deref()
    }

    pub fn connect(&mut self) -> Result<(), TlsError> {
        self.state = TlsSessionState::Connecting;
        match self.config.backend {
            TlsBackend::Mock => {
                self.negotiated_alpn = self.config.alpn.first().cloned();
                self.state = TlsSessionState::Connected;
                Ok(())
            }
            TlsBackend::Rustls | TlsBackend::Platform => {
                self.state = TlsSessionState::Failed;
                Err(TlsError::Unsupported(
                    "native TLS backend not linked in this build",
                ))
            }
        }
    }

    pub fn close(&mut self) {
        self.state = TlsSessionState::Closed;
    }
}

/// REALITY is layered on TLS-like parameters (public key / short-id elsewhere).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealityTlsOverlay {
    pub server_name: String,
    pub public_key_redacted: bool,
    pub short_id: Option<String>,
}

impl RealityTlsOverlay {
    pub fn to_tls_config(&self) -> TlsClientConfig {
        let mut c = TlsClientConfig::for_host(self.server_name.clone());
        c.backend = TlsBackend::Mock;
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Tls);
    }

    #[test]
    fn mock_handshake() {
        let mut s =
            TlsClientSession::new(TlsClientConfig::for_host("example.com").with_alpn(&["h2"]))
                .unwrap();
        s.connect().unwrap();
        assert_eq!(s.state(), TlsSessionState::Connected);
        assert_eq!(s.negotiated_alpn(), Some("h2"));
    }

    #[test]
    fn requires_sni() {
        assert!(TlsClientSession::new(TlsClientConfig::default()).is_err());
    }
}
