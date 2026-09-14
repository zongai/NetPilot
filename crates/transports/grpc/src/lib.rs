//! Grpc transport surface.
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::TransportId;
pub const CRATE_NAME: &str = "netpilot-transport-grpc";
pub fn transport_id() -> TransportId {
    TransportId::Grpc
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Grpc);
    }
}
