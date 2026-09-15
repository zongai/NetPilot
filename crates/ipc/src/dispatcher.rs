//! Command dispatcher (NP-166).

use crate::auth::{LocalAuthPolicy, PeerIdentity};
use crate::router::{RequestRouter, RouteError, RouteOutcome};
use crate::security::{authorize, redact_payload_for_log};
use crate::{ErrorBody, IpcEnvelope};

pub struct CommandDispatcher {
    router: RequestRouter,
    policy: LocalAuthPolicy,
    peer: PeerIdentity,
    enforce_auth: bool,
}

impl CommandDispatcher {
    pub fn new(router: RequestRouter) -> Self {
        Self {
            router,
            policy: LocalAuthPolicy::default(),
            peer: PeerIdentity::desktop(0, "desktop"),
            enforce_auth: false,
        }
    }

    pub fn with_auth(mut self, policy: LocalAuthPolicy, peer: PeerIdentity) -> Self {
        self.policy = policy;
        self.peer = peer;
        self.enforce_auth = true;
        self
    }

    pub fn router_mut(&mut self) -> &mut RequestRouter {
        &mut self.router
    }

    pub fn dispatch(&self, req: &IpcEnvelope) -> Result<RouteOutcome, RouteError> {
        if self.enforce_auth {
            if let Some(op) = req.operation.as_deref() {
                if !authorize(&self.policy, &self.peer, op) {
                    let resp = IpcEnvelope::error_response(
                        req.request_id.clone(),
                        req.operation.clone(),
                        ErrorBody {
                            kind: "permission_denied".into(),
                            message: "unauthorized operation".into(),
                            code: Some(403),
                        },
                    );
                    return Ok(RouteOutcome::Handled(resp));
                }
            }
        }
        let _ = req.payload.as_ref().map(redact_payload_for_log);
        self.router.dispatch(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::echo_handler;

    #[test]
    fn dispatches_echo() {
        let mut router = RequestRouter::new();
        router.register("echo", echo_handler);
        let d = CommandDispatcher::new(router);
        let req = IpcEnvelope::request("1", "echo");
        match d.dispatch(&req).unwrap() {
            RouteOutcome::Handled(_) => {}
            other => panic!("unexpected {other:?}"),
        }
    }
}
