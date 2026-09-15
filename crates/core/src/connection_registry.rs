//! Connection registry (NP-206).

use crate::connection::{ConnectionEvent, ConnectionId, ConnectionMeta, ConnectionState};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ConnectionRegistry {
    next_id: ConnectionId,
    live: HashMap<ConnectionId, (ConnectionMeta, ConnectionState)>,
    events: Vec<ConnectionEvent>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            live: HashMap::new(),
            events: Vec::new(),
        }
    }

    pub fn open(&mut self, mut meta: ConnectionMeta) -> ConnectionId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        meta.id = id;
        self.events.push(ConnectionEvent::Opened(meta.clone()));
        self.live
            .insert(id, (meta, ConnectionState::Established));
        id
    }

    pub fn close(&mut self, id: ConnectionId) -> bool {
        if self.live.remove(&id).is_some() {
            self.events.push(ConnectionEvent::Closed { id });
            true
        } else {
            false
        }
    }

    pub fn list(&self) -> Vec<&ConnectionMeta> {
        self.live.values().map(|(m, _)| m).collect()
    }

    pub fn events(&self) -> &[ConnectionEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_close() {
        let mut r = ConnectionRegistry::new();
        let id = r.open(ConnectionMeta {
            id: 0,
            destination: "a:443".into(),
            outbound: "DIRECT".into(),
            process_name: None,
            network: "tcp".into(),
        });
        assert_eq!(r.len(), 1);
        assert!(r.close(id));
        assert!(r.is_empty());
    }
}
