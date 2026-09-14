//! Resident Core service: IPC request loop over named pipe (Windows).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use netpilot_core_lib::{CoreRuntime, RuntimeState};
use netpilot_ipc::{
    register_health_handlers, HealthStatus, IpcEnvelope, MessageKind, RequestRouter, RouteOutcome,
};
use netpilot_os_pipe::{bare_name, NamedPipeListener, PipeSession, PipeTransportError};

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

    router
}

fn handle_line(router: &RequestRouter, line: &str) -> String {
    let req = match IpcEnvelope::from_json(line) {
        Ok(e) => e,
        Err(e) => {
            return IpcEnvelope::error_response(
                "invalid",
                None,
                netpilot_ipc::ErrorBody {
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
            netpilot_ipc::ErrorBody {
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
            netpilot_ipc::ErrorBody {
                kind: "internal".into(),
                message: e.to_string(),
                code: Some(500),
            },
        )
        .to_json()
        .unwrap_or_else(|_| r#"{"status":"error"}"#.into()),
    }
}

/// Serve one client connection until disconnect or stop.
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

    let mut listener = NamedPipeListener::bind(pipe)?;
    let router = build_router(runtime.state(), control.clone());

    while !control.stop_requested() {
        // Short accept timeout so we can observe stop flag.
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
                // Back off slightly on repeated errors.
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
    eprintln!("netpilot-core: idle service (no named pipe on this platform)");
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
}
