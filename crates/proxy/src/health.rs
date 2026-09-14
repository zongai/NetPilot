//! Proxy health checking (NP-032).

use std::collections::HashMap;
use std::time::Duration;

use crate::groups::MemberHealth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Unknown,
    Up,
    Down,
}

impl HealthState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Up => "up",
            Self::Down => "down",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthRecord {
    pub state: HealthState,
    pub latency_ms: Option<u32>,
    pub consecutive_fails: u32,
}

impl Default for HealthRecord {
    fn default() -> Self {
        Self {
            state: HealthState::Unknown,
            latency_ms: None,
            consecutive_fails: 0,
        }
    }
}

impl HealthRecord {
    pub fn to_member_health(&self) -> MemberHealth {
        match self.state {
            HealthState::Up => MemberHealth {
                latency_ms: self.latency_ms.or(Some(0)),
            },
            HealthState::Down | HealthState::Unknown => MemberHealth::down(),
        }
    }
}

/// In-memory health table for proxy ids (no network I/O in this task).
#[derive(Debug, Default)]
pub struct HealthTable {
    records: HashMap<String, HealthRecord>,
    /// Failures required before marking Down.
    pub fail_threshold: u32,
}

impl HealthTable {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
            fail_threshold: 3,
        }
    }

    pub fn get(&self, id: &str) -> HealthRecord {
        self.records.get(id).cloned().unwrap_or_default()
    }

    /// Record a successful probe.
    pub fn record_success(&mut self, id: &str, latency: Duration) {
        let ms = latency.as_millis().min(u32::MAX as u128) as u32;
        self.records.insert(
            id.to_string(),
            HealthRecord {
                state: HealthState::Up,
                latency_ms: Some(ms),
                consecutive_fails: 0,
            },
        );
    }

    /// Record a failed probe.
    pub fn record_failure(&mut self, id: &str) {
        let mut rec = self.get(id);
        rec.consecutive_fails = rec.consecutive_fails.saturating_add(1);
        rec.latency_ms = None;
        if rec.consecutive_fails >= self.fail_threshold {
            rec.state = HealthState::Down;
        }
        self.records.insert(id.to_string(), rec);
    }

    pub fn snapshot_for(&self, member_ids: &[String]) -> Vec<(String, MemberHealth)> {
        member_ids
            .iter()
            .map(|id| (id.clone(), self.get(id).to_member_health()))
            .collect()
    }
}

/// Probe policy parameters (actual dial is a later network task).
#[derive(Debug, Clone)]
pub struct ProbePolicy {
    pub timeout: Duration,
    pub url: String,
}

impl Default for ProbePolicy {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            url: "https://www.gstatic.com/generate_204".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_marks_up() {
        let mut t = HealthTable::new();
        t.record_success("a", Duration::from_millis(42));
        let r = t.get("a");
        assert_eq!(r.state, HealthState::Up);
        assert_eq!(r.latency_ms, Some(42));
    }

    #[test]
    fn failures_trip_threshold() {
        let mut t = HealthTable::new();
        t.fail_threshold = 2;
        t.record_failure("a");
        assert_ne!(t.get("a").state, HealthState::Down);
        t.record_failure("a");
        assert_eq!(t.get("a").state, HealthState::Down);
    }
}
