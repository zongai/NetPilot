//! NetPilot IPC (NP-015…NP-024).
//!
//! Envelope, named pipe server/client skeletons, routing, events, error map,
//! timeout/cancel, local auth, version negotiation, health endpoint.
//! No secrets in logged envelopes — callers must redact before diagnostics.

#![forbid(unsafe_code)]

mod auth;
mod client;
mod dispatcher;
mod error_map;
mod events;
mod handshake;
mod health;
mod negotiate;
mod router;
mod security;
mod server;
mod timeout;

pub use auth::{privilege_for_operation, AuthDecision, LocalAuthPolicy, PeerIdentity, Privilege};
pub use client::{ClientState, NamedPipeClient, PipeClientConfig};
pub use dispatcher::CommandDispatcher;
pub use error_map::{
    code_for_kind, ensure_error_status, error_response, map_pipe_error, map_route_error,
};
pub use events::{EventError, EventStream};
pub use handshake::{client_hello, server_handshake, HandshakeError, HandshakeResult};
pub use health::{handle_health_check, register_health_handlers, HealthStatus};
pub use negotiate::{negotiate, negotiate_response, NegotiateError, VersionOffer};
pub use router::{echo_handler, RequestRouter, RouteError, RouteOutcome};
pub use security::{authorize, redact_payload_for_log, requires_privileged};
pub use server::{
    NamedPipeServer, PipeConnection, PipeError, PipeServerConfig, ServerState, DEFAULT_PIPE_NAME,
};
pub use timeout::{check_budget, BudgetError, CancelError, CancelToken, Deadline, TimeoutError};

use serde::{Deserialize, Serialize};

/// Crate identity for diagnostics.
pub const CRATE_NAME: &str = "netpilot-ipc";

/// Current protocol version spoken by this build.
pub const PROTOCOL_VERSION: u32 = 1;

/// Minimum protocol version this build can accept.
pub const PROTOCOL_VERSION_MIN: u32 = 1;

/// Operations used by Desktop shell (NP-051+).
pub const UI_OP_HEALTH_CHECK: &str = "health.check";
pub const UI_OP_HEALTH_READY: &str = "health.ready";
pub const UI_OP_RUNTIME_STATE: &str = "runtime.state";

/// Envelope direction / role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Request,
    Response,
    Event,
}

/// Application-level status for responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusCode {
    Ok,
    Error,
}

/// Stable error payload (no secret fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Taxonomy name from `docs/ERRORS.md` (e.g. `invalid_input`).
    pub kind: String,
    /// Short safe message for logs / UI.
    pub message: String,
    /// Optional machine code for clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
}

/// Versioned IPC envelope (request, response, or event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcEnvelope {
    pub protocol_version: u32,
    pub kind: MessageKind,
    /// Correlation id shared across request/response/events.
    pub request_id: String,
    /// RPC operation name for requests; echoed on responses when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// Outcome for responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusCode>,
    /// Structured error when `status == Error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    /// Operation-specific JSON payload (opaque to the transport layer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Optional tracing / parent correlation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl IpcEnvelope {
    pub fn request(request_id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            kind: MessageKind::Request,
            request_id: request_id.into(),
            operation: Some(operation.into()),
            status: None,
            error: None,
            payload: None,
            correlation_id: None,
        }
    }

    pub fn ok_response(request_id: impl Into<String>, operation: Option<String>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            kind: MessageKind::Response,
            request_id: request_id.into(),
            operation,
            status: Some(StatusCode::Ok),
            error: None,
            payload: None,
            correlation_id: None,
        }
    }

    pub fn error_response(
        request_id: impl Into<String>,
        operation: Option<String>,
        error: ErrorBody,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            kind: MessageKind::Response,
            request_id: request_id.into(),
            operation,
            status: Some(StatusCode::Error),
            error: Some(error),
            payload: None,
            correlation_id: None,
        }
    }

    pub fn event(request_id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            kind: MessageKind::Event,
            request_id: request_id.into(),
            operation: Some(operation.into()),
            status: None,
            error: None,
            payload: None,
            correlation_id: None,
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Reject unsupported protocol versions before handling.
    pub fn validate_version(&self) -> Result<(), EnvelopeError> {
        if self.protocol_version < PROTOCOL_VERSION_MIN || self.protocol_version > PROTOCOL_VERSION
        {
            return Err(EnvelopeError::UnsupportedVersion {
                got: self.protocol_version,
                min: PROTOCOL_VERSION_MIN,
                max: PROTOCOL_VERSION,
            });
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, EnvelopeError> {
        serde_json::to_string(self).map_err(|e| EnvelopeError::Serialize(e.to_string()))
    }

    pub fn from_json(s: &str) -> Result<Self, EnvelopeError> {
        let env: Self =
            serde_json::from_str(s).map_err(|e| EnvelopeError::Deserialize(e.to_string()))?;
        env.validate_version()?;
        Ok(env)
    }
}

/// Envelope codec / validation errors (not full IPC transport errors).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    UnsupportedVersion { got: u32, min: u32, max: u32 },
    Serialize(String),
    Deserialize(String),
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion { got, min, max } => {
                write!(f, "UnsupportedVersion: got {got}, supported {min}..={max}")
            }
            Self::Serialize(msg) => write!(f, "Serialize: {msg}"),
            Self::Deserialize(msg) => write!(f, "Deserialize: {msg}"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let env = IpcEnvelope::request("req-1", "health.check")
            .with_payload(serde_json::json!({"probe": true}))
            .with_correlation("corr-9");
        let json = env.to_json().unwrap();
        let back = IpcEnvelope::from_json(&json).unwrap();
        assert_eq!(back.kind, MessageKind::Request);
        assert_eq!(back.operation.as_deref(), Some("health.check"));
        assert_eq!(back.protocol_version, PROTOCOL_VERSION);
        assert_eq!(back.correlation_id.as_deref(), Some("corr-9"));
    }

    #[test]
    fn error_response_shape() {
        let env = IpcEnvelope::error_response(
            "req-2",
            Some("config.apply".into()),
            ErrorBody {
                kind: "invalid_input".into(),
                message: "unknown field".into(),
                code: Some(400),
            },
        );
        assert_eq!(env.status, Some(StatusCode::Error));
        assert_eq!(env.error.as_ref().unwrap().kind, "invalid_input");
        let json = env.to_json().unwrap();
        assert!(!json.contains("password"));
    }

    #[test]
    fn reject_future_version() {
        let mut env = IpcEnvelope::request("r", "x");
        env.protocol_version = 99;
        let json = serde_json::to_string(&env).unwrap();
        let err = IpcEnvelope::from_json(&json).unwrap_err();
        assert!(matches!(
            err,
            EnvelopeError::UnsupportedVersion { got: 99, .. }
        ));
    }

    #[test]
    fn reject_garbage() {
        let err = IpcEnvelope::from_json("not-json").unwrap_err();
        assert!(matches!(err, EnvelopeError::Deserialize(_)));
    }

    #[test]
    fn event_kind() {
        let env = IpcEnvelope::event("evt-1", "runtime.state_changed");
        assert_eq!(env.kind, MessageKind::Event);
        env.validate_version().unwrap();
    }
}
