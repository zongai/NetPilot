//! Core log store (NP-209) — re-exports structured logging ring helpers.

use crate::logging::{redact_secrets, LogLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            target: target.into(),
            message: redact_secrets(&message.into()),
        }
    }
}

#[derive(Debug)]
pub struct LogStore {
    capacity: usize,
    entries: Vec<LogEntry>,
}

impl LogStore {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    pub fn list(&self) -> &[LogEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_on_push() {
        let mut s = LogStore::with_capacity(8);
        s.push(LogEntry::new(
            LogLevel::Info,
            "core",
            "token=supersecret&x=1",
        ));
        assert!(!s.list()[0].message.contains("supersecret"));
    }
}
