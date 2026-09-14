//! PID lifecycle handling (NP-088).

use std::collections::HashMap;

use crate::identity::ProcessIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PidEvent {
    Started { pid: u32, identity: ProcessIdentity },
    Exited { pid: u32 },
}

/// Tracks live PIDs and drops identity on exit.
#[derive(Debug, Default)]
pub struct PidTracker {
    live: HashMap<u32, ProcessIdentity>,
    events: Vec<PidEvent>,
}

impl PidTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, pid: u32, identity: ProcessIdentity) {
        self.live.insert(pid, identity.clone());
        self.events.push(PidEvent::Started { pid, identity });
    }

    pub fn note_exit(&mut self, pid: u32) {
        self.live.remove(&pid);
        self.events.push(PidEvent::Exited { pid });
    }

    pub fn get(&self, pid: u32) -> Option<&ProcessIdentity> {
        self.live.get(&pid)
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    pub fn events(&self) -> &[PidEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_removes() {
        let mut t = PidTracker::new();
        t.observe(5, ProcessIdentity::from_path(r"C:\x.exe"));
        assert_eq!(t.len(), 1);
        t.note_exit(5);
        assert!(t.is_empty());
    }
}
