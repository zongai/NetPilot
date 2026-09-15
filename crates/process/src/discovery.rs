//! Process discovery (NP-085). Mock table for CI; live Windows resolver in `win` module.

use std::collections::HashMap;

use crate::identity::ProcessIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub identity: ProcessIdentity,
}

#[derive(Debug, Default, Clone)]
pub struct MockProcessTable {
    by_pid: HashMap<u32, ProcessSnapshot>,
}

impl MockProcessTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, snap: ProcessSnapshot) {
        self.by_pid.insert(snap.pid, snap);
    }

    pub fn remove(&mut self, pid: u32) {
        self.by_pid.remove(&pid);
    }

    pub fn get(&self, pid: u32) -> Option<&ProcessSnapshot> {
        self.by_pid.get(&pid)
    }

    pub fn len(&self) -> usize {
        self.by_pid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_pid.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessResolver {
    table: MockProcessTable,
}

impl ProcessResolver {
    pub fn from_table(table: MockProcessTable) -> Self {
        Self { table }
    }

    pub fn empty() -> Self {
        Self {
            table: MockProcessTable::new(),
        }
    }

    pub fn resolve_pid(&self, pid: u32) -> Option<ProcessIdentity> {
        self.table.get(pid).map(|s| s.identity.clone())
    }

    pub fn resolve_name(&self, name: &str) -> Vec<(u32, ProcessIdentity)> {
        self.table
            .by_pid
            .iter()
            .filter(|(_, s)| s.identity.matches_name(name))
            .map(|(pid, s)| (*pid, s.identity.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_by_name() {
        let mut t = MockProcessTable::new();
        t.insert(ProcessSnapshot {
            pid: 1,
            identity: ProcessIdentity::from_path(r"C:\a\chrome.exe"),
        });
        let r = ProcessResolver::from_table(t);
        assert_eq!(r.resolve_name("chrome.exe").len(), 1);
    }
}
