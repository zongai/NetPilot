//! Websocket transport surface.
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::TransportId;
pub const CRATE_NAME: &str = "netpilot-transport-websocket";
pub fn transport_id() -> TransportId {
    TransportId::Websocket
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Websocket);
    }
}
