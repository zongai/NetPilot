//! Http2 transport surface.
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::TransportId;
pub const CRATE_NAME: &str = "netpilot-transport-http2";
pub fn transport_id() -> TransportId {
    TransportId::Http2
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Http2);
    }
}
