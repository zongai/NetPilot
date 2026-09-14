//! IPC request routing (NP-018).

use crate::{EnvelopeError, ErrorBody, IpcEnvelope, MessageKind, StatusCode};

/// Result of dispatching one request envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteOutcome {
    Handled(IpcEnvelope),
    NotFound { operation: String },
}

/// Handler for a single operation name.
pub type OperationHandler =
    Box<dyn Fn(&IpcEnvelope) -> Result<IpcEnvelope, RouteError> + Send + Sync>;

/// Simple operation → handler table.
#[derive(Default)]
pub struct RequestRouter {
    handlers: Vec<(String, OperationHandler)>,
}

impl RequestRouter {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register<F>(&mut self, operation: impl Into<String>, handler: F)
    where
        F: Fn(&IpcEnvelope) -> Result<IpcEnvelope, RouteError> + Send + Sync + 'static,
    {
        let op = operation.into();
        let boxed: OperationHandler = Box::new(handler);
        if let Some((_, h)) = self.handlers.iter_mut().find(|(k, _)| *k == op) {
            *h = boxed;
        } else {
            self.handlers.push((op, boxed));
        }
    }

    pub fn contains(&self, operation: &str) -> bool {
        self.handlers.iter().any(|(k, _)| k == operation)
    }

    /// Route a request envelope; non-requests are rejected.
    pub fn dispatch(&self, request: &IpcEnvelope) -> Result<RouteOutcome, RouteError> {
        if request.kind != MessageKind::Request {
            return Err(RouteError::InvalidInput("expected request kind"));
        }
        request
            .validate_version()
            .map_err(RouteError::from_envelope)?;
        let op = request
            .operation
            .as_deref()
            .ok_or(RouteError::InvalidInput("missing operation"))?;
        match self.handlers.iter().find(|(k, _)| k == op) {
            Some((_, handler)) => {
                let response = handler(request)?;
                if response.kind != MessageKind::Response {
                    return Err(RouteError::Internal("handler must return response"));
                }
                Ok(RouteOutcome::Handled(response))
            }
            None => Ok(RouteOutcome::NotFound {
                operation: op.to_string(),
            }),
        }
    }

    /// Build a standard not-found error response.
    pub fn not_found_response(request: &IpcEnvelope, operation: &str) -> IpcEnvelope {
        IpcEnvelope::error_response(
            request.request_id.clone(),
            Some(operation.to_string()),
            ErrorBody {
                kind: "not_found".into(),
                message: format!("unknown operation: {operation}"),
                code: Some(404),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    InvalidInput(&'static str),
    Envelope(String),
    Internal(&'static str),
}

impl RouteError {
    fn from_envelope(e: EnvelopeError) -> Self {
        Self::Envelope(e.to_string())
    }
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(m) => write!(f, "InvalidInput: {m}"),
            Self::Envelope(m) => write!(f, "Envelope: {m}"),
            Self::Internal(m) => write!(f, "Internal: {m}"),
        }
    }
}

impl std::error::Error for RouteError {}

/// Echo handler used in tests and as a sample.
pub fn echo_handler(req: &IpcEnvelope) -> Result<IpcEnvelope, RouteError> {
    Ok(IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
        .with_payload(
            req.payload
                .clone()
                .unwrap_or_else(|| serde_json::json!({})),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_registered_operation() {
        let mut router = RequestRouter::new();
        router.register("echo", echo_handler);
        let req = IpcEnvelope::request("1", "echo").with_payload(serde_json::json!({"a": 1}));
        match router.dispatch(&req).unwrap() {
            RouteOutcome::Handled(resp) => {
                assert_eq!(resp.status, Some(StatusCode::Ok));
                assert_eq!(resp.request_id, "1");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn unknown_operation() {
        let router = RequestRouter::new();
        let req = IpcEnvelope::request("2", "missing.op");
        match router.dispatch(&req).unwrap() {
            RouteOutcome::NotFound { operation } => assert_eq!(operation, "missing.op"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn rejects_non_request() {
        let router = RequestRouter::new();
        let ev = IpcEnvelope::event("3", "x");
        assert!(matches!(
            router.dispatch(&ev),
            Err(RouteError::InvalidInput(_))
        ));
    }
}
