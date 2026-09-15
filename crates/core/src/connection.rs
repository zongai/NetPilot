//! Connection model (NP-205).

use std::time::Instant;

pub type ConnectionId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Opening,
    Established,
    Closing,
    Closed,
}

impl ConnectionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opening => "opening",
            Self::Established => "established",
            Self::Closing => "closing",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionMeta {
    pub id: ConnectionId,
    pub destination: String,
    pub outbound: String,
    pub process_name: Option<String>,
    pub network: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent {
    Opened(ConnectionMeta),
    Closed { id: ConnectionId },
}

#[derive(Debug, Default)]
pub struct ConnectionIdGen {
    next: u64,
}

impl ConnectionIdGen {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    pub fn next_id(&mut self) -> ConnectionId {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }
}

/// Timestamp helper for age metrics (not exposed over IPC as Instant).
#[derive(Debug)]
pub struct ConnectionTiming {
    pub opened_at: Instant,
}

impl ConnectionTiming {
    pub fn now() -> Self {
        Self {
            opened_at: Instant::now(),
        }
    }

    pub fn age_ms(&self) -> u128 {
        self.opened_at.elapsed().as_millis()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_monotonic() {
        let mut g = ConnectionIdGen::new();
        assert_eq!(g.next_id(), 1);
        assert_eq!(g.next_id(), 2);
    }
}
