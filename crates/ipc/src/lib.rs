//! Versioned IPC envelope (NP-015).
//!
//! Transport (named pipe) is later tasks; this module defines the stable
//! request/response envelope and JSON codec. No secrets belong in envelopes
//! that are logged — callers must redact before diagnostics.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Crate identity for diagnostics.
pub const CRATE_NAME: &str = "netpilot-ipc";

/// Current protocol version spoken by this build.
pub const PROTOCOL_VERSION: u32 = 1;

/// Minimum protocol version this build can accept.
pub const PROTOCOL_VERSION_MIN: u32 = 1;

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
        if self.protocol_version < PROTOCOL_VERSION_MIN
            || self.protocol_version > PROTOCOL_VERSION
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
