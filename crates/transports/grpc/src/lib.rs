//! gRPC-over-HTTP/2 transport surface.

#![forbid(unsafe_code)]

pub use netpilot_protocol_common::TransportId;
pub use netpilot_transport_http2::{Http2ClientConfig, CLIENT_PREFACE};

pub const CRATE_NAME: &str = "netpilot-transport-grpc";

pub fn transport_id() -> TransportId {
    TransportId::Grpc
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpcClientConfig {
    pub service_name: String,
    pub authority: String,
    pub path: String,
}

impl Default for GrpcClientConfig {
    fn default() -> Self {
        Self {
            service_name: "TunService".into(),
            authority: String::new(),
            path: "/".into(),
        }
    }
}

impl GrpcClientConfig {
    pub fn http2(&self) -> Http2ClientConfig {
        Http2ClientConfig {
            host: self.authority.clone(),
            path: self.path.clone(),
            authority: self.authority.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.service_name.is_empty() {
            return Err("service_name required");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Grpc);
    }
}
