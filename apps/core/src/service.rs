#![allow(dead_code)] // pipe service path is Windows-only; exercised on target
//! Resident Core service: IPC request loop over named pipe (Windows).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use netpilot_core_lib::{CoreRuntime, RuntimeState};
use netpilot_ipc::{
    register_health_handlers, ErrorBody, HealthStatus, IpcEnvelope, MessageKind, RequestRouter,
    RouteError, RouteOutcome, DEFAULT_PIPE_NAME,
};
use netpilot_os_pipe::{bare_name, NamedPipeListener, PipeSession, PipeTransportError};
use netpilot_subscription::{
    run_subscription_pipeline, FilterRule, RenameRule, SubscriptionFetcher, SubscriptionManager,
    SubscriptionProfile,
};

#[cfg(not(feature = "real-http"))]
use netpilot_subscription::MockFetcher;
#[cfg(feature = "real-http")]
use netpilot_subscription::UreqFetcher;

/// Shared flag so IPC `runtime.shutdown` can stop the accept loop.
pub struct ServiceControl {
    pub stop: AtomicBool,
}

impl ServiceControl {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            stop: AtomicBool::new(false),
        })
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

fn make_fetcher() -> Box<dyn SubscriptionFetcher + Send> {
    #[cfg(feature = "real-http")]
    {
        // Core binary enables the feature via Cargo.toml dependency features.
        Box::new(UreqFetcher::new())
    }
    #[cfg(not(feature = "real-http"))]
    {
        Box::new(MockFetcher::new())
    }
}

/// Whether this Core build can perform outbound HTTP for subscriptions.
fn http_mode() -> &'static str {
    #[cfg(feature = "real-http")]
    {
        "real-http"
    }
    #[cfg(not(feature = "real-http"))]
    {
        "mock"
    }
}

fn err_resp(
    req: &IpcEnvelope,
    kind: &str,
    message: String,
    code: i32,
) -> Result<IpcEnvelope, RouteError> {
    Ok(IpcEnvelope::error_response(
        req.request_id.clone(),
        req.operation.clone(),
        ErrorBody {
            kind: kind.into(),
            message,
            code: Some(code),
        },
    ))
}

fn build_router(runtime_state: RuntimeState, control: Arc<ServiceControl>) -> RequestRouter {
    let mut router = RequestRouter::new();
    let health = HealthStatus::new(
        runtime_state.as_str(),
        runtime_state == RuntimeState::Running,
    );
    register_health_handlers(&mut router, health);

    let state = runtime_state;
    router.register("runtime.state", move |req| {
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "state": state.as_str(),
                }),
            ),
        )
    });

    let control_shutdown = control.clone();
    router.register("runtime.shutdown", move |req| {
        control_shutdown.request_stop();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "accepted": true,
                }),
            ),
        )
    });

    router.register("ping", move |req| {
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "pong": true })),
        )
    });

    // Shared subscription store for this Core process.
    let mgr: Arc<Mutex<SubscriptionManager>> = Arc::new(Mutex::new(SubscriptionManager::new()));
    let fetcher: Arc<Mutex<Box<dyn SubscriptionFetcher + Send>>> =
        Arc::new(Mutex::new(make_fetcher()));

    let mgr_list = mgr.clone();
    router.register("subscription.list", move |req| {
        let guard = mgr_list
            .lock()
            .map_err(|_| RouteError::Internal("subscription lock poisoned"))?;
        let items: Vec<serde_json::Value> = guard
            .list()
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "url": p.url,
                    "state": format!("{:?}", p.state),
                    "enabled": p.enabled,
                })
            })
            .collect();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "items": items,
                    "http_mode": http_mode(),
                }),
            ),
        )
    });

    let mgr_add = mgr.clone();
    router.register("subscription.add", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let url = payload
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.url required"))?;
        let mut profile = SubscriptionProfile::new(id, name, url);
        if let Some(false) = payload.get("enabled").and_then(|v| v.as_bool()) {
            profile.enabled = false;
        }
        let mut guard = mgr_add
            .lock()
            .map_err(|_| RouteError::Internal("subscription lock poisoned"))?;
        match guard.upsert(profile) {
            Ok(()) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({ "id": id, "accepted": true })),
            ),
            Err(e) => err_resp(req, "invalid_argument", e.to_string(), 400),
        }
    });

    let mgr_upd = mgr.clone();
    let fetcher_upd = fetcher.clone();
    router.register("subscription.update", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.id required"))?;
        let timeout_secs = payload
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);
        let timeout = Duration::from_secs(timeout_secs.max(1));

        let mut mgr_guard = mgr_upd
            .lock()
            .map_err(|_| RouteError::Internal("subscription lock poisoned"))?;
        let mut fetcher_guard = fetcher_upd
            .lock()
            .map_err(|_| RouteError::Internal("fetcher lock poisoned"))?;

        match mgr_guard.update_one(id, fetcher_guard.as_mut(), timeout) {
            Ok(body) => {
                // Parse into ProxyProfile list for caller convenience.
                let profile = mgr_guard.get(id).cloned();
                let nodes = if let Some(p) = profile.as_ref() {
                    let pipe = run_subscription_pipeline(
                        p,
                        &body,
                        &FilterRule::default(),
                        &RenameRule::default(),
                    );
                    pipe.profiles
                        .iter()
                        .map(|n| {
                            serde_json::json!({
                                "name": n.name,
                                "server": n.server,
                                "port": n.port,
                                "protocol": format!("{:?}", n.protocol),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                Ok(
                    IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                        .with_payload(serde_json::json!({
                            "id": id,
                            "bytes": body.len(),
                            "nodes": nodes,
                            "node_count": nodes.len(),
                            "http_mode": http_mode(),
                        })),
                )
            }
            Err(e) => err_resp(req, "failed_precondition", e.to_string(), 502),
        }
    });

    let mgr_rm = mgr.clone();
    router.register("subscription.remove", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.id required"))?;
        let mut guard = mgr_rm
            .lock()
            .map_err(|_| RouteError::Internal("subscription lock poisoned"))?;
        let removed = guard.remove(id);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "id": id, "removed": removed })),
        )
    });

    router
}

fn handle_line(router: &RequestRouter, line: &str) -> String {
    let req = match IpcEnvelope::from_json(line) {
        Ok(e) => e,
        Err(e) => {
            return IpcEnvelope::error_response(
                "invalid",
                None,
                ErrorBody {
                    kind: "invalid_argument".into(),
                    message: e.to_string(),
                    code: Some(400),
                },
            )
            .to_json()
            .unwrap_or_else(|_| r#"{"status":"error"}"#.into());
        }
    };

    if req.kind != MessageKind::Request {
        return IpcEnvelope::error_response(
            req.request_id,
            req.operation,
            ErrorBody {
                kind: "invalid_argument".into(),
                message: "expected request kind".into(),
                code: Some(400),
            },
        )
        .to_json()
        .unwrap_or_else(|_| r#"{"status":"error"}"#.into());
    }

    match router.dispatch(&req) {
        Ok(RouteOutcome::Handled(resp)) => resp
            .to_json()
            .unwrap_or_else(|_| r#"{"status":"error"}"#.into()),
        Ok(RouteOutcome::NotFound { operation }) => {
            RequestRouter::not_found_response(&req, &operation)
                .to_json()
                .unwrap_or_else(|_| r#"{"status":"error"}"#.into())
        }
        Err(e) => IpcEnvelope::error_response(
            req.request_id,
            req.operation,
            ErrorBody {
                kind: "internal".into(),
                message: e.to_string(),
                code: Some(500),
            },
        )
        .to_json()
        .unwrap_or_else(|_| r#"{"status":"error"}"#.into()),
    }
}

fn serve_session(session: &mut dyn PipeSession, router: &RequestRouter, control: &ServiceControl) {
    let read_timeout = Duration::from_secs(300);
    loop {
        if control.stop_requested() {
            break;
        }
        match session.read_line(read_timeout) {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                let response = handle_line(router, &line);
                if let Err(e) = session.write_line(&response) {
                    eprintln!("netpilot-core: write failed: {e}");
                    break;
                }
            }
            Err(PipeTransportError::Timeout) => continue,
            Err(PipeTransportError::Disconnected) => break,
            Err(e) => {
                eprintln!("netpilot-core: read failed: {e}");
                break;
            }
        }
    }
    session.close();
}

/// Run named-pipe accept loop until shutdown requested.
pub fn run_pipe_service(
    runtime: &CoreRuntime,
    control: Arc<ServiceControl>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pipe = DEFAULT_PIPE_NAME;
    eprintln!(
        "netpilot-core: listening on \\\\.\\pipe\\{} (bare={})",
        bare_name(pipe),
        bare_name(pipe)
    );
    eprintln!("netpilot-core: subscription http_mode={}", http_mode());

    let mut listener = NamedPipeListener::bind(pipe)?;
    let router = build_router(runtime.state(), control.clone());

    while !control.stop_requested() {
        match listener.accept(Duration::from_secs(2)) {
            Ok(mut session) => {
                eprintln!("netpilot-core: client connected");
                serve_session(&mut session, &router, &control);
                eprintln!("netpilot-core: client disconnected");
            }
            Err(PipeTransportError::Timeout) => continue,
            Err(PipeTransportError::Unsupported(m)) => {
                eprintln!("netpilot-core: pipe unsupported: {m}");
                return Err(m.into());
            }
            Err(e) => {
                eprintln!("netpilot-core: accept error: {e}");
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }

    Ok(())
}

/// Non-Windows / CI: idle until stop or optional max duration.
pub fn run_idle_service(
    _runtime: &CoreRuntime,
    control: Arc<ServiceControl>,
    max_idle: Option<Duration>,
) {
    eprintln!(
        "netpilot-core: idle service (no named pipe); subscription http_mode={}",
        http_mode()
    );
    let started = std::time::Instant::now();
    while !control.stop_requested() {
        if let Some(max) = max_idle {
            if started.elapsed() > max {
                eprintln!("netpilot-core: idle deadline reached");
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_ipc::StatusCode;

    #[test]
    fn router_ping() {
        let control = ServiceControl::new();
        let router = build_router(RuntimeState::Running, control);
        let req = IpcEnvelope::request("1", "ping");
        match router.dispatch(&req).unwrap() {
            RouteOutcome::Handled(resp) => assert_eq!(resp.status, Some(StatusCode::Ok)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn subscription_add_list() {
        let control = ServiceControl::new();
        let router = build_router(RuntimeState::Running, control);
        let add = IpcEnvelope::request("a1", "subscription.add").with_payload(serde_json::json!({
            "id": "s1",
            "name": "Demo",
            "url": "https://example.com/sub"
        }));
        match router.dispatch(&add).unwrap() {
            RouteOutcome::Handled(resp) => assert_eq!(resp.status, Some(StatusCode::Ok)),
            other => panic!("{other:?}"),
        }
        let list = IpcEnvelope::request("a2", "subscription.list");
        match router.dispatch(&list).unwrap() {
            RouteOutcome::Handled(resp) => {
                assert_eq!(resp.status, Some(StatusCode::Ok));
                let items = resp.payload.as_ref().unwrap()["items"].as_array().unwrap();
                assert_eq!(items.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }
}
