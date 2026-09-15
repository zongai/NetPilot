//! IPC handshake (NP-163).

use crate::auth::{AuthDecision, LocalAuthPolicy, PeerIdentity, Privilege};
use crate::negotiate::{negotiate, NegotiateError, VersionOffer};
use crate::{IpcEnvelope, PROTOCOL_VERSION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeResult {
    pub protocol_version: u32,
    pub peer: PeerIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    Negotiate(NegotiateError),
    AuthDenied { reason: &'static str },
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negotiate(e) => write!(f, "negotiate: {e}"),
            Self::AuthDenied { reason } => write!(f, "auth denied: {reason}"),
        }
    }
}

impl std::error::Error for HandshakeError {}

/// Server-side handshake: version overlap + local peer write privilege.
pub fn server_handshake(
    client_offer: VersionOffer,
    peer: PeerIdentity,
    policy: &LocalAuthPolicy,
) -> Result<HandshakeResult, HandshakeError> {
    let server = VersionOffer::current();
    let ver = negotiate(&client_offer, &server).map_err(HandshakeError::Negotiate)?;
    match policy.authorize(&peer, Privilege::Write) {
        AuthDecision::Allow => Ok(HandshakeResult {
            protocol_version: ver,
            peer,
        }),
        AuthDecision::Deny { reason } => Err(HandshakeError::AuthDenied { reason }),
    }
}

pub fn client_hello() -> IpcEnvelope {
    IpcEnvelope::request("handshake-1", "ipc.handshake").with_payload(serde_json::json!({
        "min": crate::PROTOCOL_VERSION_MIN,
        "max": PROTOCOL_VERSION,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_desktop() {
        let peer = PeerIdentity::desktop(1, "NetPilot.Desktop.exe");
        let policy = LocalAuthPolicy::default();
        let r = server_handshake(VersionOffer::current(), peer, &policy).unwrap();
        assert_eq!(r.protocol_version, PROTOCOL_VERSION);
    }
}
