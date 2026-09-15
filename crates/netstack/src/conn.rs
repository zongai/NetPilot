//! TCP connection table linking 4-tuples to outbound streams.

use std::collections::HashMap;
use std::time::Instant;

use crate::tcp::TcpState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FourTuple {
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub sport: u16,
    pub dport: u16,
}

#[derive(Debug)]
pub struct TcpConn {
    pub state: TcpState,
    pub client_seq: u32,
    pub server_seq: u32,
    pub outbound_name: String,
    pub created: Instant,
    pub bytes_up: u64,
    pub bytes_down: u64,
}

#[derive(Debug, Default)]
pub struct ConnTable {
    tcp: HashMap<FourTuple, TcpConn>,
}

impl ConnTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_mut(&mut self, key: &FourTuple) -> Option<&mut TcpConn> {
        self.tcp.get_mut(key)
    }

    pub fn insert(&mut self, key: FourTuple, conn: TcpConn) {
        self.tcp.insert(key, conn);
    }

    pub fn remove(&mut self, key: &FourTuple) -> Option<TcpConn> {
        self.tcp.remove(key)
    }

    pub fn len(&self) -> usize {
        self.tcp.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tcp.is_empty()
    }

    pub fn retain_active(&mut self, max_age_secs: u64) {
        let now = Instant::now();
        self.tcp.retain(|_, c| {
            now.duration_since(c.created).as_secs() < max_age_secs && c.state != TcpState::Closed
        });
    }
}
