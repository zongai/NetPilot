//! Core health / readiness endpoint (NP-024).

use crate::router::{RequestRouter, RouteError};
use crate::{IpcEnvelope, StatusCode, PROTOCOL_VERSION};

/// Snapshot exposed to Desktop via `health.check` / `health.ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    pub runtime_state: String,
    pub ready: bool,
    pub protocol_version: u32,
}

impl HealthStatus {
    pub fn new(runtime_state: impl Into<String>, ready: bool) -> Self {
        Self {
            runtime_state: runtime_state.into(),
            ready,
            protocol_version: PROTOCOL_VERSION,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "runtime_state": self.runtime_state,
            "ready": self.ready,
            "protocol_version": self.protocol_version,
        })
    }
}

/// Register health operations on a router with a fixed snapshot.
pub fn register_health_handlers(router: &mut RequestRouter, status: HealthStatus) {
    let status_check = status.clone();
    let status_ready = status;
    router.register("health.check", move |req| {
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(status_check.to_json()),
        )
    });
    router.register("health.ready", move |req| {
        let mut env = IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
            .with_payload(serde_json::json!({ "ready": status_ready.ready }));
        if !status_ready.ready {
            env.status = Some(StatusCode::Error);
            env.error = Some(crate::ErrorBody {
                kind: "failed_precondition".into(),
                message: "core not ready".into(),
                code: Some(412),
            });
        }
        Ok(env)
    });
}

pub fn handle_health_check(
    req: &IpcEnvelope,
    status: &HealthStatus,
) -> Result<IpcEnvelope, RouteError> {
    Ok(
        IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
            .with_payload(status.to_json()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::RouteOutcome;

    #[test]
    fn health_when_running() {
        let status = HealthStatus::new("running", true);
        let mut router = RequestRouter::new();
        register_health_handlers(&mut router, status);
        let req = IpcEnvelope::request("h1", "health.check");
        match router.dispatch(&req).unwrap() {
            RouteOutcome::Handled(resp) => {
                assert_eq!(resp.status, Some(StatusCode::Ok));
                assert_eq!(resp.payload.as_ref().unwrap()["runtime_state"], "running");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ready_fails_when_not_running() {
        let status = HealthStatus::new("starting", false);
        let mut router = RequestRouter::new();
        register_health_handlers(&mut router, status);
        let req = IpcEnvelope::request("h2", "health.ready");
        match router.dispatch(&req).unwrap() {
            RouteOutcome::Handled(resp) => {
                assert_eq!(resp.status, Some(StatusCode::Error));
            }
            other => panic!("{other:?}"),
        }
    }
}
