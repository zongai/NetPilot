//! Core health surface (NP-147).
//!
//! Process-local health snapshot for IPC `health` / diagnostics. No secrets.

use crate::RuntimeState;
use std::time::{Duration, Instant};

/// Coarse health level exposed to Desktop and smoke runners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthLevel {
    Unknown,
    Starting,
    Ready,
    Degraded,
    Stopping,
    Failed,
}

impl HealthLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
        }
    }

    pub fn from_runtime(state: RuntimeState) -> Self {
        match state {
            RuntimeState::Created => Self::Unknown,
            RuntimeState::Starting => Self::Starting,
            RuntimeState::Running => Self::Ready,
            RuntimeState::Stopping => Self::Stopping,
            RuntimeState::Stopped => Self::Unknown,
            RuntimeState::Failed => Self::Failed,
        }
    }
}

/// Snapshot used by IPC and Desktop status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreHealth {
    pub level: HealthLevel,
    pub state: RuntimeState,
    pub uptime: Duration,
    pub ipc_bound: bool,
    pub message: String,
}

impl CoreHealth {
    pub fn new(state: RuntimeState, started_at: Instant, ipc_bound: bool, message: impl Into<String>) -> Self {
        Self {
            level: HealthLevel::from_runtime(state),
            state,
            uptime: started_at.elapsed(),
            ipc_bound,
            message: message.into(),
        }
    }

    pub fn ready(started_at: Instant, ipc_bound: bool) -> Self {
        Self::new(
            RuntimeState::Running,
            started_at,
            ipc_bound,
            if ipc_bound {
                "core ready"
            } else {
                "core running (ipc not bound)"
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_from_state() {
        assert_eq!(
            HealthLevel::from_runtime(RuntimeState::Running),
            HealthLevel::Ready
        );
        assert_eq!(
            HealthLevel::from_runtime(RuntimeState::Failed),
            HealthLevel::Failed
        );
    }

    #[test]
    fn ready_snapshot() {
        let h = CoreHealth::ready(Instant::now(), true);
        assert_eq!(h.level, HealthLevel::Ready);
        assert!(h.ipc_bound);
        assert_eq!(h.message, "core ready");
    }
}
