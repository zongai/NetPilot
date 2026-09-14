//! Connection event model, manager, metadata, lifecycle (NP-097…NP-100).

use std::collections::HashMap;
use std::time::Instant;

pub type ConnectionId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Opening,
    Established,
    Closing,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionMeta {
    pub id: ConnectionId,
    pub process_name: Option<String>,
    pub destination: String,
    pub network: String,
    pub outbound: String,
    pub rule_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent {
    Opened(ConnectionMeta),
    Updated { id: ConnectionId, outbound: String },
    Closed { id: ConnectionId },
}

#[derive(Debug)]
struct LiveConnection {
    meta: ConnectionMeta,
    state: ConnectionState,
    opened_at: Instant,
    bytes_up: u64,
    bytes_down: u64,
}

#[derive(Debug, Default)]
pub struct ConnectionManager {
    next_id: ConnectionId,
    live: HashMap<ConnectionId, LiveConnection>,
    events: Vec<ConnectionEvent>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
        &mut self,
        destination: impl Into<String>,
        network: impl Into<String>,
        process_name: Option<String>,
        outbound: impl Into<String>,
        rule_summary: Option<String>,
    ) -> ConnectionId {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        let meta = ConnectionMeta {
            id,
            process_name,
            destination: destination.into(),
            network: network.into(),
            outbound: outbound.into(),
            rule_summary,
        };
        self.live.insert(
            id,
            LiveConnection {
                meta: meta.clone(),
                state: ConnectionState::Established,
                opened_at: Instant::now(),
                bytes_up: 0,
                bytes_down: 0,
            },
        );
        self.events.push(ConnectionEvent::Opened(meta));
        id
    }

    pub fn add_bytes(&mut self, id: ConnectionId, up: u64, down: u64) {
        if let Some(c) = self.live.get_mut(&id) {
            c.bytes_up = c.bytes_up.saturating_add(up);
            c.bytes_down = c.bytes_down.saturating_add(down);
        }
    }

    pub fn close(&mut self, id: ConnectionId) {
        if let Some(mut c) = self.live.remove(&id) {
            c.state = ConnectionState::Closed;
            self.events.push(ConnectionEvent::Closed { id });
        }
    }

    pub fn get(&self, id: ConnectionId) -> Option<&ConnectionMeta> {
        self.live.get(&id).map(|c| &c.meta)
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    pub fn events(&self) -> &[ConnectionEvent] {
        &self.events
    }

    pub fn list(&self) -> Vec<ConnectionMeta> {
        self.live.values().map(|c| c.meta.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_close() {
        let mut m = ConnectionManager::new();
        let id = m.open(
            "1.1.1.1:443",
            "tcp",
            Some("chrome.exe".into()),
            "PROXY",
            None,
        );
        assert_eq!(m.len(), 1);
        m.close(id);
        assert!(m.is_empty());
    }
}
