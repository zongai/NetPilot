//! Core runtime orchestration types (NP-013).
//!
//! Owns the high-level lifecycle state machine. IPC, TUN, and adapters
//! plug in during later tasks; this crate stays free of Windows-only APIs.

#![forbid(unsafe_code)]

/// Crate identity for diagnostics.
pub const CRATE_NAME: &str = "netpilot-core-lib";

/// High-level Core process lifecycle.
///
/// Transitions are explicit; illegal moves return [`Error::FailedPrecondition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeState {
    /// Process constructed; subsystems not started.
    Created,
    /// Starting IPC / internal services.
    Starting,
    /// Ready to accept IPC and run workloads.
    Running,
    /// Graceful shutdown in progress.
    Stopping,
    /// Terminal state; instance must not be restarted in-place.
    Stopped,
    /// Unrecoverable failure recorded; requires new instance or explicit reset policy (later).
    Failed,
}

impl RuntimeState {
    /// Human-stable name for logs / IPC (no localization).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

/// Error kinds aligned with `docs/ERRORS.md` (subset used by the state machine).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    FailedPrecondition {
        from: RuntimeState,
        to: RuntimeState,
        message: &'static str,
    },
    Cancelled,
    Internal(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedPrecondition { from, to, message } => write!(
                f,
                "FailedPrecondition: cannot transition {} -> {}: {}",
                from.as_str(),
                to.as_str(),
                message
            ),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Internal(msg) => write!(f, "Internal: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Core runtime state machine.
#[derive(Debug)]
pub struct CoreRuntime {
    state: RuntimeState,
    cancel_requested: bool,
}

impl CoreRuntime {
    /// Create a runtime in [`RuntimeState::Created`].
    pub fn new() -> Self {
        Self {
            state: RuntimeState::Created,
            cancel_requested: false,
        }
    }

    pub fn state(&self) -> RuntimeState {
        self.state
    }

    /// Request cooperative cancellation (e.g. Ctrl+C or IPC stop).
    pub fn request_cancel(&mut self) {
        self.cancel_requested = true;
    }

    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_requested
    }

    /// `Created -> Starting`.
    pub fn begin_start(&mut self) -> Result<(), Error> {
        self.transition(
            RuntimeState::Created,
            RuntimeState::Starting,
            "begin_start",
        )
    }

    /// `Starting -> Running` (after subsystems report ready in later tasks).
    pub fn mark_running(&mut self) -> Result<(), Error> {
        if self.cancel_requested {
            return Err(Error::Cancelled);
        }
        self.transition(
            RuntimeState::Starting,
            RuntimeState::Running,
            "mark_running",
        )
    }

    /// Enter graceful stop from `Running` or `Starting`.
    pub fn begin_stop(&mut self) -> Result<(), Error> {
        match self.state {
            RuntimeState::Running | RuntimeState::Starting => {
                self.state = RuntimeState::Stopping;
                Ok(())
            }
            other => Err(Error::FailedPrecondition {
                from: other,
                to: RuntimeState::Stopping,
                message: "begin_stop requires Starting or Running",
            }),
        }
    }

    /// `Stopping -> Stopped`.
    pub fn mark_stopped(&mut self) -> Result<(), Error> {
        self.transition(
            RuntimeState::Stopping,
            RuntimeState::Stopped,
            "mark_stopped",
        )
    }

    /// Record unrecoverable failure from non-terminal states.
    pub fn mark_failed(&mut self) -> Result<(), Error> {
        match self.state {
            RuntimeState::Stopped | RuntimeState::Failed => Err(Error::FailedPrecondition {
                from: self.state,
                to: RuntimeState::Failed,
                message: "already terminal",
            }),
            _ => {
                self.state = RuntimeState::Failed;
                Ok(())
            }
        }
    }

    fn transition(
        &mut self,
        expected: RuntimeState,
        next: RuntimeState,
        op: &'static str,
    ) -> Result<(), Error> {
        if self.state != expected {
            return Err(Error::FailedPrecondition {
                from: self.state,
                to: next,
                message: op,
            });
        }
        self.state = next;
        Ok(())
    }
}

impl Default for CoreRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_start_stop() {
        let mut rt = CoreRuntime::new();
        assert_eq!(rt.state(), RuntimeState::Created);
        rt.begin_start().unwrap();
        assert_eq!(rt.state(), RuntimeState::Starting);
        rt.mark_running().unwrap();
        assert_eq!(rt.state(), RuntimeState::Running);
        rt.begin_stop().unwrap();
        assert_eq!(rt.state(), RuntimeState::Stopping);
        rt.mark_stopped().unwrap();
        assert_eq!(rt.state(), RuntimeState::Stopped);
    }

    #[test]
    fn reject_running_before_start() {
        let mut rt = CoreRuntime::new();
        let err = rt.mark_running().unwrap_err();
        assert!(matches!(err, Error::FailedPrecondition { .. }));
    }

    #[test]
    fn cancel_during_start() {
        let mut rt = CoreRuntime::new();
        rt.begin_start().unwrap();
        rt.request_cancel();
        assert!(matches!(rt.mark_running(), Err(Error::Cancelled)));
    }

    #[test]
    fn stop_from_starting() {
        let mut rt = CoreRuntime::new();
        rt.begin_start().unwrap();
        rt.begin_stop().unwrap();
        rt.mark_stopped().unwrap();
        assert_eq!(rt.state(), RuntimeState::Stopped);
    }

    #[test]
    fn failed_from_running() {
        let mut rt = CoreRuntime::new();
        rt.begin_start().unwrap();
        rt.mark_running().unwrap();
        rt.mark_failed().unwrap();
        assert_eq!(rt.state(), RuntimeState::Failed);
        assert!(rt.mark_failed().is_err());
    }

    #[test]
    fn state_as_str_stable() {
        assert_eq!(RuntimeState::Running.as_str(), "running");
    }
}
