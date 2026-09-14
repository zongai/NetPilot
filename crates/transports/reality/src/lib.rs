//! Reality transport surface.
#![forbid(unsafe_code)]
pub use netpilot_protocol_common::TransportId;
pub const CRATE_NAME: &str = "netpilot-transport-reality";
pub fn transport_id() -> TransportId {
    TransportId::Reality
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Reality);
    }
}
