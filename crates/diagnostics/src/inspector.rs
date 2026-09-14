//! Inspector API contract (NP-102).

use crate::connection::{ConnectionManager, ConnectionMeta};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectorSnapshot {
    pub connection_count: usize,
    pub connections: Vec<ConnectionMeta>,
}

pub trait InspectorApi {
    fn snapshot(&self) -> InspectorSnapshot;
    fn get(&self, id: u64) -> Option<ConnectionMeta>;
}

impl InspectorApi for ConnectionManager {
    fn snapshot(&self) -> InspectorSnapshot {
        let connections = self.list();
        InspectorSnapshot {
            connection_count: connections.len(),
            connections,
        }
    }

    fn get(&self, id: u64) -> Option<ConnectionMeta> {
        ConnectionManager::get(self, id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionManager;

    #[test]
    fn snapshot() {
        let mut m = ConnectionManager::new();
        m.open("x:80", "tcp", None, "DIRECT", None);
        let s = InspectorApi::snapshot(&m);
        assert_eq!(s.connection_count, 1);
    }
}
