//! IPC compatibility / version negotiation (NP-023).

use crate::{IpcEnvelope, PROTOCOL_VERSION, PROTOCOL_VERSION_MIN};

/// Client advertises what it speaks; server responds with chosen version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionOffer {
    pub min: u32,
    pub max: u32,
}

impl VersionOffer {
    pub fn current() -> Self {
        Self {
            min: PROTOCOL_VERSION_MIN,
            max: PROTOCOL_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiateError {
    NoOverlap {
        client: VersionOffer,
        server: VersionOffer,
    },
    InvalidRange,
}

impl std::fmt::Display for NegotiateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOverlap { client, server } => write!(
                f,
                "NoOverlap: client {}..={} server {}..={}",
                client.min, client.max, server.min, server.max
            ),
            Self::InvalidRange => write!(f, "InvalidRange"),
        }
    }
}

impl std::error::Error for NegotiateError {}

/// Pick the highest mutually supported protocol version.
pub fn negotiate(client: &VersionOffer, server: &VersionOffer) -> Result<u32, NegotiateError> {
    if client.min > client.max || server.min > server.max {
        return Err(NegotiateError::InvalidRange);
    }
    let lo = client.min.max(server.min);
    let hi = client.max.min(server.max);
    if lo > hi {
        return Err(NegotiateError::NoOverlap {
            client: client.clone(),
            server: server.clone(),
        });
    }
    Ok(hi)
}

/// Build a negotiation response payload envelope.
pub fn negotiate_response(request_id: &str, chosen: u32) -> IpcEnvelope {
    IpcEnvelope::ok_response(request_id, Some("ipc.negotiate".into())).with_payload(
        serde_json::json!({
            "protocol_version": chosen,
            "min": PROTOCOL_VERSION_MIN,
            "max": PROTOCOL_VERSION,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_highest_overlap() {
        let client = VersionOffer { min: 1, max: 2 };
        let server = VersionOffer { min: 1, max: 1 };
        assert_eq!(negotiate(&client, &server).unwrap(), 1);
    }

    #[test]
    fn no_overlap() {
        let client = VersionOffer { min: 2, max: 3 };
        let server = VersionOffer { min: 1, max: 1 };
        assert!(matches!(
            negotiate(&client, &server),
            Err(NegotiateError::NoOverlap { .. })
        ));
    }

    #[test]
    fn invalid_range() {
        let bad = VersionOffer { min: 3, max: 1 };
        assert!(matches!(
            negotiate(&bad, &VersionOffer::current()),
            Err(NegotiateError::InvalidRange)
        ));
    }
}
