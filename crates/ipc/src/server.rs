//! Named pipe server skeleton (NP-016).
//!
//! Platform pipe creation is deferred to a later wiring task; this module owns
//! configuration, validation, listen/accept state machine, timeouts, and cancel.

use std::time::Duration;

/// Default local pipe name for Core (Windows path form).
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\netpilot-core";

/// Server lifecycle for the pipe endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    Created,
    Listening,
    Connected,
    ShuttingDown,
    Closed,
}

impl ServerState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Listening => "listening",
            Self::Connected => "connected",
            Self::ShuttingDown => "shutting_down",
            Self::Closed => "closed",
        }
    }
}

/// Server configuration (no secrets).
#[derive(Debug, Clone)]
pub struct PipeServerConfig {
    /// Windows pipe path, e.g. `\\.\pipe\netpilot-core`.
    pub pipe_name: String,
    /// Accept / I/O deadline.
    pub accept_timeout: Duration,
    /// Max concurrent instances (documented for later OS binding).
    pub max_instances: u32,
}

impl Default for PipeServerConfig {
    fn default() -> Self {
        Self {
            pipe_name: DEFAULT_PIPE_NAME.to_string(),
            accept_timeout: Duration::from_secs(5),
            max_instances: 1,
        }
    }
}

impl PipeServerConfig {
    pub fn with_pipe_name(mut self, name: impl Into<String>) -> Self {
        self.pipe_name = name.into();
        self
    }

    pub fn with_accept_timeout(mut self, timeout: Duration) -> Self {
        self.accept_timeout = timeout;
        self
    }

    /// Validate Windows-style pipe path without touching the OS.
    pub fn validate(&self) -> Result<(), PipeError> {
        if self.pipe_name.is_empty() {
            return Err(PipeError::InvalidInput("pipe_name empty"));
        }
        let lower = self.pipe_name.to_ascii_lowercase();
        if !lower.starts_with(r"\\.\pipe\") && !lower.starts_with(r"//./pipe/") {
            return Err(PipeError::InvalidInput(
                "pipe_name must use \\\\.\\pipe\\ or //./pipe/ prefix",
            ));
        }
        if self.pipe_name.len() > 256 {
            return Err(PipeError::InvalidInput("pipe_name too long"));
        }
        if self.max_instances == 0 {
            return Err(PipeError::InvalidInput("max_instances must be >= 1"));
        }
        if self.accept_timeout.is_zero() {
            return Err(PipeError::InvalidInput("accept_timeout must be > 0"));
        }
        Ok(())
    }
}

/// Pipe server / session errors (aligned with docs/ERRORS.md naming).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipeError {
    InvalidInput(&'static str),
    FailedPrecondition(&'static str),
    Timeout { op: &'static str },
    Cancelled,
    Unavailable(&'static str),
    /// OS bind not wired yet on this build/target.
    Unimplemented(&'static str),
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(m) => write!(f, "InvalidInput: {m}"),
            Self::FailedPrecondition(m) => write!(f, "FailedPrecondition: {m}"),
            Self::Timeout { op } => write!(f, "Timeout: {op}"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Unavailable(m) => write!(f, "Unavailable: {m}"),
            Self::Unimplemented(m) => write!(f, "Unimplemented: {m}"),
        }
    }
}

impl std::error::Error for PipeError {}

/// Named pipe server skeleton (Core side).
#[derive(Debug)]
pub struct NamedPipeServer {
    config: PipeServerConfig,
    state: ServerState,
    cancel_requested: bool,
    /// When true, accept uses an in-process test double instead of OS pipes.
    test_mode: bool,
    /// Simulated client waiting (test_mode only).
    test_client_pending: bool,
}

impl NamedPipeServer {
    pub fn new(config: PipeServerConfig) -> Result<Self, PipeError> {
        config.validate()?;
        Ok(Self {
            config,
            state: ServerState::Created,
            cancel_requested: false,
            test_mode: false,
            test_client_pending: false,
        })
    }

    /// Construct a server that never touches the OS (unit tests).
    pub fn new_test(config: PipeServerConfig) -> Result<Self, PipeError> {
        let mut s = Self::new(config)?;
        s.test_mode = true;
        Ok(s)
    }

    pub fn state(&self) -> ServerState {
        self.state
    }

    pub fn pipe_name(&self) -> &str {
        &self.config.pipe_name
    }

    pub fn request_cancel(&mut self) {
        self.cancel_requested = true;
    }

    /// Enter listening state. OS `CreateNamedPipe` is future work; test_mode succeeds.
    pub fn listen(&mut self) -> Result<(), PipeError> {
        if self.cancel_requested {
            return Err(PipeError::Cancelled);
        }
        if self.state != ServerState::Created && self.state != ServerState::Closed {
            return Err(PipeError::FailedPrecondition(
                "listen requires Created or Closed",
            ));
        }
        self.config.validate()?;
        if self.test_mode {
            self.state = ServerState::Listening;
            return Ok(());
        }
        // Production Windows bind lands with a dedicated transport task.
        // Keep the state transition explicit so Core lifecycle can call listen().
        #[cfg(windows)]
        {
            // Skeleton: mark listening without creating a real handle yet.
            // Real CreateNamedPipeW will replace this without changing the API.
            self.state = ServerState::Listening;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(PipeError::Unimplemented(
                "named pipe listen requires Windows or test_mode",
            ))
        }
    }

    /// Simulate a client connecting (test_mode only).
    pub fn test_signal_client(&mut self) -> Result<(), PipeError> {
        if !self.test_mode {
            return Err(PipeError::FailedPrecondition(
                "test_signal_client only in test_mode",
            ));
        }
        if self.state != ServerState::Listening {
            return Err(PipeError::FailedPrecondition(
                "client signal requires Listening",
            ));
        }
        self.test_client_pending = true;
        Ok(())
    }

    /// Accept one client with timeout and cancel support.
    pub fn accept(&mut self) -> Result<PipeConnection, PipeError> {
        if self.cancel_requested {
            return Err(PipeError::Cancelled);
        }
        if self.state != ServerState::Listening {
            return Err(PipeError::FailedPrecondition("accept requires Listening"));
        }
        if self.test_mode {
            if self.test_client_pending {
                self.test_client_pending = false;
                self.state = ServerState::Connected;
                return Ok(PipeConnection {
                    pipe_name: self.config.pipe_name.clone(),
                    test_mode: true,
                });
            }
            // No client: honor timeout without blocking the test thread.
            return Err(PipeError::Timeout { op: "accept" });
        }
        #[cfg(windows)]
        {
            // Skeleton without blocking the CI agent on a real pipe wait.
            Err(PipeError::Unimplemented(
                "OS accept not wired; use test_mode or later NP task",
            ))
        }
        #[cfg(not(windows))]
        {
            Err(PipeError::Unimplemented("named pipe accept requires Windows"))
        }
    }

    /// Graceful shutdown; safe from any non-Closed state.
    pub fn shutdown(&mut self) -> Result<(), PipeError> {
        if self.state == ServerState::Closed {
            return Ok(());
        }
        self.state = ServerState::ShuttingDown;
        self.test_client_pending = false;
        self.state = ServerState::Closed;
        Ok(())
    }
}

/// Accepted pipe session skeleton (read/write envelopes in later tasks).
#[derive(Debug)]
pub struct PipeConnection {
    pipe_name: String,
    test_mode: bool,
}

impl PipeConnection {
    pub fn pipe_name(&self) -> &str {
        &self.pipe_name
    }

    pub fn is_test_mode(&self) -> bool {
        self.test_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_valid() {
        PipeServerConfig::default().validate().unwrap();
    }

    #[test]
    fn reject_bad_pipe_name() {
        let cfg = PipeServerConfig::default().with_pipe_name("not-a-pipe");
        assert!(matches!(cfg.validate(), Err(PipeError::InvalidInput(_))));
    }

    #[test]
    fn reject_zero_timeout() {
        let cfg = PipeServerConfig {
            accept_timeout: Duration::from_millis(0),
            ..PipeServerConfig::default()
        };
        assert!(matches!(cfg.validate(), Err(PipeError::InvalidInput(_))));
    }

    #[test]
    fn listen_accept_shutdown_test_mode() {
        let mut server = NamedPipeServer::new_test(PipeServerConfig::default()).unwrap();
        assert_eq!(server.state(), ServerState::Created);
        server.listen().unwrap();
        assert_eq!(server.state(), ServerState::Listening);
        // no client
        assert!(matches!(
            server.accept(),
            Err(PipeError::Timeout { op: "accept" })
        ));
        server.test_signal_client().unwrap();
        let conn = server.accept().unwrap();
        assert!(conn.is_test_mode());
        assert_eq!(server.state(), ServerState::Connected);
        server.shutdown().unwrap();
        assert_eq!(server.state(), ServerState::Closed);
    }

    #[test]
    fn cancel_before_listen() {
        let mut server = NamedPipeServer::new_test(PipeServerConfig::default()).unwrap();
        server.request_cancel();
        assert!(matches!(server.listen(), Err(PipeError::Cancelled)));
    }

    #[test]
    fn accept_without_listen_fails() {
        let mut server = NamedPipeServer::new_test(PipeServerConfig::default()).unwrap();
        assert!(matches!(
            server.accept(),
            Err(PipeError::FailedPrecondition(_))
        ));
    }

    #[test]
    fn forward_slash_pipe_prefix_ok() {
        let cfg = PipeServerConfig::default().with_pipe_name("//./pipe/netpilot-core");
        cfg.validate().unwrap();
    }
}
