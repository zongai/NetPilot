//! Graceful shutdown coordinator (NP-148).
//!
//! Tracks cooperative stop reasons and drains ordered phases without secrets.

use crate::{Error, StopPhase};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Why shutdown was requested (stable for logs / IPC; never includes credentials).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShutdownReason {
    Explicit,
    IpcCommand,
    Signal,
    Timeout,
    Fatal,
}

impl ShutdownReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::IpcCommand => "ipc_command",
            Self::Signal => "signal",
            Self::Timeout => "timeout",
            Self::Fatal => "fatal",
        }
    }
}

/// Shared stop flag + reason + deadline.
#[derive(Debug)]
pub struct ShutdownCoordinator {
    stop: AtomicBool,
    reason: std::sync::Mutex<Option<ShutdownReason>>,
    deadline: std::sync::Mutex<Option<Instant>>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            stop: AtomicBool::new(false),
            reason: std::sync::Mutex::new(None),
            deadline: std::sync::Mutex::new(None),
        }
    }

    pub fn request(&self, reason: ShutdownReason, budget: Duration) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut g) = self.reason.lock() {
            if g.is_none() {
                *g = Some(reason);
            }
        }
        if let Ok(mut d) = self.deadline.lock() {
            if d.is_none() {
                *d = Some(Instant::now() + budget);
            }
        }
    }

    pub fn is_requested(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn reason(&self) -> Option<ShutdownReason> {
        self.reason.lock().ok().and_then(|g| *g)
    }

    pub fn is_past_deadline(&self) -> bool {
        self.deadline
            .lock()
            .ok()
            .and_then(|g| *g)
            .is_some_and(|t| Instant::now() >= t)
    }

    /// Run stop phases with a budget; returns Timeout if exceeded.
    pub fn run_phases<F>(&self, budget: Duration, mut hook: F) -> Result<(), Error>
    where
        F: FnMut(StopPhase) -> Result<(), Error>,
    {
        let started = Instant::now();
        for phase in StopPhase::all() {
            if started.elapsed() > budget || self.is_past_deadline() {
                return Err(Error::Timeout {
                    op: "shutdown",
                    budget,
                });
            }
            hook(*phase)?;
        }
        Ok(())
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_sets_flag_and_reason() {
        let c = ShutdownCoordinator::new();
        assert!(!c.is_requested());
        c.request(ShutdownReason::IpcCommand, Duration::from_secs(5));
        assert!(c.is_requested());
        assert_eq!(c.reason(), Some(ShutdownReason::IpcCommand));
    }

    #[test]
    fn phases_run_in_order() {
        let c = ShutdownCoordinator::new();
        let mut seen = Vec::new();
        c.run_phases(Duration::from_secs(5), |p| {
            seen.push(p.as_str());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            vec!["drain_ipc", "stop_subsystems", "release_resources", "final"]
        );
    }
}
