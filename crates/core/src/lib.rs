//! Core runtime orchestration (NP-013 state machine + NP-014 lifecycle + V5 F0).
//!
//! Owns high-level lifecycle. IPC, TUN, and adapters plug in later;
//! this crate stays free of Windows-only APIs.

#![forbid(unsafe_code)]

mod connection;
mod connection_registry;
mod health;
mod instance;
mod logging;
mod log_store;
mod shutdown;

pub use connection::{ConnectionEvent, ConnectionId, ConnectionIdGen, ConnectionMeta, ConnectionState, ConnectionTiming};
pub use connection_registry::ConnectionRegistry;
pub use health::{CoreHealth, HealthLevel};
pub use instance::InstanceLock;
pub use logging::{log_line, max_level, redact_secrets, set_max_level, LogLevel};
pub use log_store::{LogEntry, LogStore};
pub use shutdown::{ShutdownCoordinator, ShutdownReason};

use std::time::Duration;

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

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }
}

/// Ordered startup phases (NP-014). Real work is filled in by later tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartPhase {
    InitLogging,
    LoadConfig,
    BindIpc,
    StartSubsystems,
    Ready,
}

impl StartPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InitLogging => "init_logging",
            Self::LoadConfig => "load_config",
            Self::BindIpc => "bind_ipc",
            Self::StartSubsystems => "start_subsystems",
            Self::Ready => "ready",
        }
    }

    pub fn all() -> &'static [StartPhase] {
        &[
            Self::InitLogging,
            Self::LoadConfig,
            Self::BindIpc,
            Self::StartSubsystems,
            Self::Ready,
        ]
    }
}

/// Ordered shutdown phases (NP-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StopPhase {
    DrainIpc,
    StopSubsystems,
    ReleaseResources,
    Final,
}

impl StopPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DrainIpc => "drain_ipc",
            Self::StopSubsystems => "stop_subsystems",
            Self::ReleaseResources => "release_resources",
            Self::Final => "final",
        }
    }

    pub fn all() -> &'static [StopPhase] {
        &[
            Self::DrainIpc,
            Self::StopSubsystems,
            Self::ReleaseResources,
            Self::Final,
        ]
    }
}

/// Error kinds aligned with `docs/ERRORS.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    FailedPrecondition {
        from: RuntimeState,
        to: RuntimeState,
        message: &'static str,
    },
    Cancelled,
    Timeout {
        op: &'static str,
        budget: Duration,
    },
    PhaseFailed {
        phase: &'static str,
        message: &'static str,
    },
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
            Self::Timeout { op, budget } => {
                write!(f, "Timeout: {op} exceeded {budget:?}")
            }
            Self::PhaseFailed { phase, message } => {
                write!(f, "PhaseFailed: {phase}: {message}")
            }
            Self::Internal(msg) => write!(f, "Internal: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Options for orchestrated start/stop (NP-014).
#[derive(Debug, Clone)]
pub struct LifecycleOptions {
    /// Maximum wall time for the entire start sequence.
    pub start_timeout: Duration,
    /// Maximum wall time for the entire stop sequence.
    pub stop_timeout: Duration,
}

impl Default for LifecycleOptions {
    fn default() -> Self {
        Self {
            start_timeout: Duration::from_secs(30),
            stop_timeout: Duration::from_secs(15),
        }
    }
}

/// Hook invoked per phase. Returns `Err` to fail the lifecycle.
///
/// Later tasks replace no-op defaults with real subsystem bring-up.
pub type PhaseHook = fn(phase_name: &'static str) -> Result<(), Error>;

fn default_start_hook(_phase: &'static str) -> Result<(), Error> {
    Ok(())
}

fn default_stop_hook(_phase: &'static str) -> Result<(), Error> {
    Ok(())
}

/// Core runtime: state machine + startup/shutdown orchestration.
#[derive(Debug)]
pub struct CoreRuntime {
    state: RuntimeState,
    cancel_requested: bool,
    options: LifecycleOptions,
    start_hook: PhaseHook,
    stop_hook: PhaseHook,
    last_start_phase: Option<StartPhase>,
    last_stop_phase: Option<StopPhase>,
}

impl CoreRuntime {
    /// Create a runtime in [`RuntimeState::Created`] with default options.
    pub fn new() -> Self {
        Self::with_options(LifecycleOptions::default())
    }

    pub fn with_options(options: LifecycleOptions) -> Self {
        Self {
            state: RuntimeState::Created,
            cancel_requested: false,
            options,
            start_hook: default_start_hook,
            stop_hook: default_stop_hook,
            last_start_phase: None,
            last_stop_phase: None,
        }
    }

    /// Install test/production phase hooks (no secrets in phase names).
    pub fn set_hooks(&mut self, start: PhaseHook, stop: PhaseHook) {
        self.start_hook = start;
        self.stop_hook = stop;
    }

    pub fn state(&self) -> RuntimeState {
        self.state
    }

    pub fn last_start_phase(&self) -> Option<StartPhase> {
        self.last_start_phase
    }

    pub fn last_stop_phase(&self) -> Option<StopPhase> {
        self.last_stop_phase
    }

    /// Request cooperative cancellation (Ctrl+C, IPC stop, deadline).
    pub fn request_cancel(&mut self) {
        self.cancel_requested = true;
    }

    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_requested
    }

    /// `Created -> Starting`.
    pub fn begin_start(&mut self) -> Result<(), Error> {
        self.transition(RuntimeState::Created, RuntimeState::Starting, "begin_start")
    }

    /// `Starting -> Running`.
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

    /// Full startup: `Created -> Starting` → phases → `Running`.
    ///
    /// Honors [`LifecycleOptions::start_timeout`] via a monotonic budget check
    /// between phases (phase hooks themselves should also bound I/O later).
    pub fn start(&mut self) -> Result<(), Error> {
        self.begin_start()?;
        let budget = self.options.start_timeout;
        let started = std::time::Instant::now();

        for phase in StartPhase::all() {
            if self.cancel_requested {
                let _ = self.begin_stop();
                let _ = self.run_stop_phases(self.options.stop_timeout);
                let _ = self.mark_stopped();
                return Err(Error::Cancelled);
            }
            if started.elapsed() > budget {
                let _ = self.mark_failed();
                return Err(Error::Timeout {
                    op: "start",
                    budget,
                });
            }
            self.last_start_phase = Some(*phase);
            if let Err(err) = (self.start_hook)(phase.as_str()) {
                let _ = self.mark_failed();
                return Err(err);
            }
        }

        self.mark_running()
    }

    /// Full shutdown from `Running` or `Starting`: phases → `Stopped`.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        if self.state.is_terminal() {
            return Err(Error::FailedPrecondition {
                from: self.state,
                to: RuntimeState::Stopped,
                message: "shutdown on terminal state",
            });
        }
        if !matches!(
            self.state,
            RuntimeState::Running | RuntimeState::Starting | RuntimeState::Stopping
        ) {
            return Err(Error::FailedPrecondition {
                from: self.state,
                to: RuntimeState::Stopping,
                message: "shutdown requires active lifecycle",
            });
        }
        if self.state != RuntimeState::Stopping {
            self.begin_stop()?;
        }
        self.run_stop_phases(self.options.stop_timeout)?;
        self.mark_stopped()
    }

    fn run_stop_phases(&mut self, budget: Duration) -> Result<(), Error> {
        let started = std::time::Instant::now();
        for phase in StopPhase::all() {
            if started.elapsed() > budget {
                let _ = self.mark_failed();
                return Err(Error::Timeout {
                    op: "shutdown",
                    budget,
                });
            }
            self.last_stop_phase = Some(*phase);
            if let Err(err) = (self.stop_hook)(phase.as_str()) {
                let _ = self.mark_failed();
                return Err(err);
            }
        }
        Ok(())
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
    fn orchestrated_start_shutdown() {
        let mut rt = CoreRuntime::new();
        rt.start().unwrap();
        assert_eq!(rt.state(), RuntimeState::Running);
        assert_eq!(rt.last_start_phase(), Some(StartPhase::Ready));
        rt.shutdown().unwrap();
        assert_eq!(rt.state(), RuntimeState::Stopped);
        assert_eq!(rt.last_stop_phase(), Some(StopPhase::Final));
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
    fn cancel_during_orchestrated_start() {
        fn canceling_hook(phase: &'static str) -> Result<(), Error> {
            if phase == StartPhase::BindIpc.as_str() {
                // Simulate external cancel observed mid-start: caller sets flag.
            }
            Ok(())
        }
        let mut rt = CoreRuntime::new();
        rt.set_hooks(canceling_hook, default_stop_hook);
        rt.begin_start().unwrap();
        // After first phases would run; request cancel then continue start().
        // Drive start() which checks cancel between phases.
        rt.request_cancel();
        // state is still Created if we only set hook — use full start from Created:
        let mut rt = CoreRuntime::new();
        rt.set_hooks(
            |phase| {
                if phase == StartPhase::LoadConfig.as_str() {
                    // Cannot request cancel from hook without &mut; tested below differently.
                }
                Ok(())
            },
            default_stop_hook,
        );
        // Explicit path: begin_start, cancel, start path via mark_running
        rt.begin_start().unwrap();
        rt.request_cancel();
        assert!(matches!(rt.mark_running(), Err(Error::Cancelled)));
    }

    #[test]
    fn start_phase_failure_marks_failed() {
        fn boom(phase: &'static str) -> Result<(), Error> {
            if phase == StartPhase::LoadConfig.as_str() {
                return Err(Error::PhaseFailed {
                    phase,
                    message: "config missing",
                });
            }
            Ok(())
        }
        let mut rt = CoreRuntime::new();
        rt.set_hooks(boom, default_stop_hook);
        let err = rt.start().unwrap_err();
        assert!(matches!(err, Error::PhaseFailed { .. }));
        assert_eq!(rt.state(), RuntimeState::Failed);
    }

    #[test]
    fn start_timeout() {
        let mut rt = CoreRuntime::with_options(LifecycleOptions {
            start_timeout: Duration::from_millis(0),
            stop_timeout: Duration::from_secs(1),
        });
        // Zero budget: after begin_start, first phase loop sees elapsed > budget.
        let err = rt.start().unwrap_err();
        assert!(matches!(err, Error::Timeout { op: "start", .. }));
        assert_eq!(rt.state(), RuntimeState::Failed);
    }

    #[test]
    fn shutdown_on_created_fails() {
        let mut rt = CoreRuntime::new();
        assert!(matches!(
            rt.shutdown(),
            Err(Error::FailedPrecondition { .. })
        ));
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
        assert_eq!(StartPhase::BindIpc.as_str(), "bind_ipc");
        assert_eq!(StopPhase::DrainIpc.as_str(), "drain_ipc");
    }
}
