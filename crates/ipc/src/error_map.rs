//! IPC error mapping (NP-020) — map internal kinds to envelope ErrorBody.

use crate::router::RouteError;
use crate::server::PipeError;
use crate::{ErrorBody, IpcEnvelope, StatusCode};

/// Map a taxonomy kind + message into a response envelope.
pub fn error_response(
    request_id: impl Into<String>,
    operation: Option<String>,
    kind: &str,
    message: impl Into<String>,
    code: Option<i32>,
) -> IpcEnvelope {
    IpcEnvelope::error_response(
        request_id,
        operation,
        ErrorBody {
            kind: kind.to_string(),
            message: message.into(),
            code,
        },
    )
}

/// Stable numeric codes for common kinds.
pub fn code_for_kind(kind: &str) -> i32 {
    match kind {
        "invalid_input" => 400,
        "not_found" => 404,
        "conflict" => 409,
        "permission_denied" => 403,
        "failed_precondition" => 412,
        "timeout" => 408,
        "cancelled" => 499,
        "unavailable" => 503,
        "unimplemented" => 501,
        "internal" => 500,
        _ => 500,
    }
}

pub fn map_pipe_error(request_id: &str, operation: Option<String>, err: &PipeError) -> IpcEnvelope {
    let (kind, message) = match err {
        PipeError::InvalidInput(m) => ("invalid_input", *m),
        PipeError::FailedPrecondition(m) => ("failed_precondition", *m),
        PipeError::Timeout { op } => ("timeout", *op),
        PipeError::Cancelled => ("cancelled", "cancelled"),
        PipeError::Unavailable(m) => ("unavailable", *m),
        PipeError::Unimplemented(m) => ("unimplemented", *m),
    };
    error_response(
        request_id,
        operation,
        kind,
        message,
        Some(code_for_kind(kind)),
    )
}

pub fn map_route_error(
    request_id: &str,
    operation: Option<String>,
    err: &RouteError,
) -> IpcEnvelope {
    let (kind, message) = match err {
        RouteError::InvalidInput(m) => ("invalid_input", (*m).to_string()),
        RouteError::Envelope(m) => ("invalid_input", m.clone()),
        RouteError::Internal(m) => ("internal", (*m).to_string()),
    };
    let code = code_for_kind(kind);
    error_response(request_id, operation, kind, message, Some(code))
}

/// Ensure response carries Error status when ErrorBody is present.
pub fn ensure_error_status(env: &IpcEnvelope) -> bool {
    matches!(env.status, Some(StatusCode::Error)) && env.error.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_timeout() {
        let env = map_pipe_error("r", Some("op".into()), &PipeError::Timeout { op: "accept" });
        assert!(ensure_error_status(&env));
        assert_eq!(env.error.as_ref().unwrap().kind, "timeout");
        assert_eq!(env.error.as_ref().unwrap().code, Some(408));
    }

    #[test]
    fn maps_route_invalid() {
        let env = map_route_error("r", None, &RouteError::InvalidInput("missing operation"));
        assert_eq!(env.error.as_ref().unwrap().kind, "invalid_input");
        assert_eq!(env.error.as_ref().unwrap().code, Some(400));
    }

    #[test]
    fn code_table() {
        assert_eq!(code_for_kind("permission_denied"), 403);
        assert_eq!(code_for_kind("unknown_xyz"), 500);
    }
}
