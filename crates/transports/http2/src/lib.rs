//! HTTP/2 transport surface (connection preface + settings skeleton).

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;

pub const CRATE_NAME: &str = "netpilot-transport-http2";

pub fn transport_id() -> TransportId {
    TransportId::Http2
}

/// Client connection preface magic.
pub const CLIENT_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Http2ClientConfig {
    pub host: String,
    pub path: String,
    pub authority: String,
}

impl Default for Http2ClientConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            path: "/".into(),
            authority: String::new(),
        }
    }
}

impl Http2ClientConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.authority.is_empty() && self.host.is_empty() {
            return Err("authority or host required");
        }
        Ok(())
    }

    pub fn preface_and_settings(&self) -> Vec<u8> {
        let mut out = CLIENT_PREFACE.to_vec();
        // Empty SETTINGS frame: length=0 type=4 flags=0 stream=0
        out.extend_from_slice(&[0, 0, 0, 0x04, 0, 0, 0, 0, 0]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preface_starts_with_pri() {
        let c = Http2ClientConfig {
            host: "h".into(),
            path: "/".into(),
            authority: "h".into(),
        };
        let b = c.preface_and_settings();
        assert!(b.starts_with(b"PRI * HTTP/2.0"));
    }
}
