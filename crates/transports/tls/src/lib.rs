//! Tls transport surface.
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::TransportId;
pub const CRATE_NAME: &str = "netpilot-transport-tls";
pub fn transport_id() -> TransportId {
    TransportId::Tls
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Tls);
    }
}
