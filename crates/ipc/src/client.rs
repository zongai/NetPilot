//! Named pipe client skeleton (NP-017).

use std::time::Duration;

use crate::server::{PipeError, DEFAULT_PIPE_NAME};

/// Client connection lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Created,
    Connecting,
    Connected,
    Closed,
}

impl ClientState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Closed => "closed",
        }
    }
}

/// Client configuration (no secrets).
#[derive(Debug, Clone)]
pub struct PipeClientConfig {
    pub pipe_name: String,
    pub connect_timeout: Duration,
}

impl Default for PipeClientConfig {
    fn default() -> Self {
        Self {
            pipe_name: DEFAULT_PIPE_NAME.to_string(),
            connect_timeout: Duration::from_secs(5),
        }
    }
}

impl PipeClientConfig {
    pub fn with_pipe_name(mut self, name: impl Into<String>) -> Self {
        self.pipe_name = name.into();
        self
    }

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
        if self.connect_timeout.is_zero() {
            return Err(PipeError::InvalidInput("connect_timeout must be > 0"));
        }
        Ok(())
    }
}

/// Named pipe client skeleton (Desktop side).
#[derive(Debug)]
pub struct NamedPipeClient {
    config: PipeClientConfig,
    state: ClientState,
    cancel_requested: bool,
    test_mode: bool,
    /// When set, connect succeeds without OS (test_mode).
    test_server_ready: bool,
}

impl NamedPipeClient {
    pub fn new(config: PipeClientConfig) -> Result<Self, PipeError> {
        config.validate()?;
        Ok(Self {
            config,
            state: ClientState::Created,
            cancel_requested: false,
            test_mode: false,
            test_server_ready: false,
        })
    }

    pub fn new_test(config: PipeClientConfig) -> Result<Self, PipeError> {
        let mut c = Self::new(config)?;
        c.test_mode = true;
        Ok(c)
    }

    pub fn state(&self) -> ClientState {
        self.state
    }

    pub fn pipe_name(&self) -> &str {
        &self.config.pipe_name
    }

    pub fn request_cancel(&mut self) {
        self.cancel_requested = true;
    }

    /// Mark that a test server is accepting (test_mode only).
    pub fn test_set_server_ready(&mut self, ready: bool) -> Result<(), PipeError> {
        if !self.test_mode {
            return Err(PipeError::FailedPrecondition(
                "test_set_server_ready only in test_mode",
            ));
        }
        self.test_server_ready = ready;
        Ok(())
    }

    /// Connect with timeout / cancel. OS connect is later; test_mode is deterministic.
    pub fn connect(&mut self) -> Result<(), PipeError> {
        if self.cancel_requested {
            return Err(PipeError::Cancelled);
        }
        if self.state != ClientState::Created && self.state != ClientState::Closed {
            return Err(PipeError::FailedPrecondition(
                "connect requires Created or Closed",
            ));
        }
        self.state = ClientState::Connecting;
        if self.test_mode {
            if self.test_server_ready {
                self.state = ClientState::Connected;
                return Ok(());
            }
            self.state = ClientState::Created;
            return Err(PipeError::Timeout { op: "connect" });
        }
        #[cfg(windows)]
        {
            self.state = ClientState::Created;
            Err(PipeError::Unimplemented(
                "OS client connect not wired; use test_mode or later NP task",
            ))
        }
        #[cfg(not(windows))]
        {
            self.state = ClientState::Created;
            Err(PipeError::Unimplemented(
                "named pipe client requires Windows or test_mode",
            ))
        }
    }

    pub fn close(&mut self) -> Result<(), PipeError> {
        self.state = ClientState::Closed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_when_server_ready() {
        let mut client = NamedPipeClient::new_test(PipeClientConfig::default()).unwrap();
        client.test_set_server_ready(true).unwrap();
        client.connect().unwrap();
        assert_eq!(client.state(), ClientState::Connected);
        client.close().unwrap();
        assert_eq!(client.state(), ClientState::Closed);
    }

    #[test]
    fn connect_timeout_without_server() {
        let mut client = NamedPipeClient::new_test(PipeClientConfig::default()).unwrap();
        let err = client.connect().unwrap_err();
        assert!(matches!(err, PipeError::Timeout { op: "connect" }));
    }

    #[test]
    fn cancel_before_connect() {
        let mut client = NamedPipeClient::new_test(PipeClientConfig::default()).unwrap();
        client.request_cancel();
        assert!(matches!(client.connect(), Err(PipeError::Cancelled)));
    }

    #[test]
    fn reject_bad_name() {
        let cfg = PipeClientConfig::default().with_pipe_name("bad");
        assert!(matches!(cfg.validate(), Err(PipeError::InvalidInput(_))));
    }
}
