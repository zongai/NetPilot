#![allow(dead_code)] // pipe service path is Windows-only; exercised on target
//! Resident Core service: IPC request loop over named pipe (Windows).

use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use netpilot_config::ConfigDocument;
use netpilot_core_lib::{CoreRuntime, RuntimeState};
use netpilot_diagnostics::ConnectionManager;
use netpilot_dns::{DnsQuery, SystemResolver};
use netpilot_engine::{start_socks_inbound, TrafficEngine};
use netpilot_ipc::{
    register_health_handlers, ErrorBody, HealthStatus, IpcEnvelope, MessageKind, RequestRouter,
    RouteError, RouteOutcome, DEFAULT_PIPE_NAME,
};
use netpilot_os_pipe::{bare_name, NamedPipeListener, PipeSession, PipeTransportError};
use netpilot_os_proxy::{
    apply_system_proxy, disable_system_proxy, query_system_proxy, AutoProxyMode, SystemProxyAuto,
    SystemProxySettings,
};
use netpilot_proxy::{ProtocolKind, ProxyProfile};
use netpilot_subscription::{
    run_subscription_pipeline, FilterRule, RenameRule, SubscriptionFetcher, SubscriptionManager,
    SubscriptionProfile,
};
use netpilot_transport_tcp::{dial_tcp, TcpDialRequest};
use netpilot_tun::{WintunDllPath, WintunSession};

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

    let engine: Arc<Mutex<TrafficEngine>> = Arc::new(Mutex::new(TrafficEngine::new()));
    let inbound: Arc<Mutex<Option<netpilot_engine::SocksInbound>>> = Arc::new(Mutex::new(None));
    let conn_mgr: Arc<Mutex<ConnectionManager>> = Arc::new(Mutex::new(ConnectionManager::new()));
    let pump_stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let sys_proxy: Arc<Mutex<SystemProxyAuto>> = Arc::new(Mutex::new(SystemProxyAuto::new()));

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
    let engine_sub = engine.clone();
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
                if let Some(p) = mgr_guard.get(id).cloned() {
                    let pipe = run_subscription_pipeline(
                        &p,
                        &body,
                        &FilterRule::default(),
                        &RenameRule::default(),
                    );
                    if let Ok(mut eng) = engine_sub.lock() {
                        for n in pipe.profiles {
                            eng.add_profile(n);
                        }
                    }
                }
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

    // --- Traffic engine (rules + profiles + TUN + outbound) ---
    let engine_load = engine.clone();
    router.register("rules.load", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let text = payload
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.text required"))?;
        let mut g = engine_load
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.load_rules_text(text) {
            Ok(count) => Ok(IpcEnvelope::ok_response(
                req.request_id.clone(),
                req.operation.clone(),
            )
            .with_payload(serde_json::json!({ "loaded": count }))),
            Err(e) => err_resp(req, "invalid_argument", e.to_string(), 400),
        }
    });

    let engine_decide = engine.clone();
    router.register("rules.decide", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let domain = payload.get("domain").and_then(|v| v.as_str());
        let ip = payload.get("ip").and_then(|v| v.as_str());
        let port = payload
            .get("port")
            .and_then(|v| v.as_u64())
            .map(|p| p as u16);
        let g = engine_decide
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        let r = g.decide(domain, ip, port);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "outbound": r.outbound,
                    "explanation": r.explanation,
                    "matcher": r.matcher,
                }),
            ),
        )
    });

    let engine_dial = engine.clone();
    router.register("traffic.route_dial", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let host = payload
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.host required"))?;
        let port = payload
            .get("port")
            .and_then(|v| v.as_u64())
            .ok_or(RouteError::InvalidInput("payload.port required"))? as u16;
        let mut g = engine_dial
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.route_and_dial(host, port) {
            Ok((route, report)) => Ok(IpcEnvelope::ok_response(
                req.request_id.clone(),
                req.operation.clone(),
            )
            .with_payload(serde_json::json!({
                "outbound": route.outbound,
                "explanation": route.explanation,
                "matcher": route.matcher,
                "protocol": report.protocol,
                "server": report.server,
                "peer": report.peer,
                "elapsed_ms": report.elapsed_ms,
                "via": report.via,
            }))),
            Err(e) => err_resp(req, "unavailable", e.to_string(), 503),
        }
    });

    let engine_prof = engine.clone();
    router.register("proxy.upsert", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("id"))?;
        let name = payload.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let server = payload
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("server"))?;
        let port = payload
            .get("port")
            .and_then(|v| v.as_u64())
            .ok_or(RouteError::InvalidInput("port"))? as u16;
        let proto_s = payload
            .get("protocol")
            .and_then(|v| v.as_str())
            .unwrap_or("socks5");
        let protocol =
            ProtocolKind::parse(proto_s).ok_or(RouteError::InvalidInput("unknown protocol"))?;
        let profile = ProxyProfile {
            id: id.into(),
            name: name.into(),
            protocol,
            transport: None,
            server: server.into(),
            port,
            password: payload
                .get("password")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            uuid: payload
                .get("uuid")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            username: payload
                .get("username")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            sni: payload
                .get("sni")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            alpn: payload
                .get("alpn")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            path: None,
            host: None,
            flow: None,
            network: None,
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        };
        let mut g = engine_prof
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        g.add_profile(profile);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "id": id, "accepted": true })),
        )
    });

    let engine_sel = engine.clone();
    router.register("proxy.select", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let id = payload
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("id"))?;
        let mut g = engine_sel
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        g.select_outbound(id);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "selected": id })),
        )
    });

    let engine_list = engine.clone();
    router.register("proxy.list", move |req| {
        let g = engine_list
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        let items: Vec<serde_json::Value> = g
            .profiles()
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "server": p.server,
                    "port": p.port,
                    "protocol": p.protocol.as_str(),
                })
            })
            .collect();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "items": items,
                    "selected": g.selected_outbound(),
                }),
            ),
        )
    });

    let engine_tun_start = engine.clone();
    router.register("tunnel.start", move |req| {
        let name = req
            .payload
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str());
        let mut g = engine_tun_start
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.start_tunnel(name) {
            Ok(st) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({
                        "running": st.running,
                        "native": st.native,
                        "adapter": st.adapter,
                        "state": st.state,
                        "last_error": st.last_error,
                        "adapter_luid": st.adapter_luid,
                        "configured_ip": st.configured_ip,
                    })),
            ),
            Err(e) => err_resp(req, "failed_precondition", e.to_string(), 500),
        }
    });

    let engine_tun_stop = engine.clone();
    router.register("tunnel.stop", move |req| {
        let mut g = engine_tun_stop
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.stop_tunnel() {
            Ok(st) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({
                        "running": st.running,
                        "state": st.state,
                    })),
            ),
            Err(e) => err_resp(req, "internal", e.to_string(), 500),
        }
    });

    let engine_tun_st = engine.clone();
    router.register("tunnel.status", move |req| {
        let g = engine_tun_st
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        let st = g.tunnel_status();
        let stats = g.stats();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "running": st.running,
                    "native": st.native,
                    "adapter": st.adapter,
                    "state": st.state,
                    "packets_in": st.packets_in,
                    "packets_out": st.packets_out,
                    "last_error": st.last_error,
                    "adapter_luid": st.adapter_luid,
                    "configured_ip": st.configured_ip,
                    "stats": {
                        "routed": stats.routed,
                        "direct": stats.direct,
                        "proxy": stats.proxy,
                        "reject": stats.reject,
                        "dial_fail": stats.dial_fail,
                    }
                }),
            ),
        )
    });
    let engine_in = engine.clone();
    let inbound_start = inbound.clone();
    let sys_proxy_start = sys_proxy.clone();
    router.register("inbound.socks_start", move |req| {
        let port = req
            .payload
            .as_ref()
            .and_then(|p| p.get("port"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;
        let auto_system = req
            .payload
            .as_ref()
            .and_then(|p| p.get("system_proxy"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mut slot = inbound_start
            .lock()
            .map_err(|_| RouteError::Internal("inbound lock poisoned"))?;
        if let Some(existing) = slot.as_ref() {
            return Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({
                        "port": existing.port(),
                        "already_running": true,
                    })),
            );
        }
        match start_socks_inbound(engine_in.clone(), port) {
            Ok(ib) => {
                let bound = ib.port();
                *slot = Some(ib);
                let mut sys_applied = false;
                if auto_system {
                    if let Ok(mut sp) = sys_proxy_start.lock() {
                        if sp
                            .enable_local(AutoProxyMode::Socks, "127.0.0.1", bound)
                            .is_ok()
                        {
                            sys_applied = true;
                        }
                    }
                }
                Ok(
                    IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                        .with_payload(serde_json::json!({
                            "port": bound,
                            "listen": format!("127.0.0.1:{bound}"),
                            "already_running": false,
                            "system_proxy": sys_applied,
                        })),
                )
            }
            Err(e) => err_resp(req, "failed_precondition", e, 500),
        }
    });

    let inbound_stop = inbound.clone();
    let sys_proxy_stop = sys_proxy.clone();
    router.register("inbound.socks_stop", move |req| {
        let mut slot = inbound_stop
            .lock()
            .map_err(|_| RouteError::Internal("inbound lock poisoned"))?;
        if let Some(ib) = slot.take() {
            ib.stop();
        }
        if let Ok(mut sp) = sys_proxy_stop.lock() {
            let _ = sp.disable();
        }
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "stopped": true })),
        )
    });

    let inbound_st = inbound.clone();
    router.register("inbound.socks_status", move |req| {
        let slot = inbound_st
            .lock()
            .map_err(|_| RouteError::Internal("inbound lock poisoned"))?;
        match slot.as_ref() {
            Some(ib) => {
                let s = ib.stats_snapshot();
                Ok(
                    IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                        .with_payload(serde_json::json!({
                            "running": true,
                            "port": ib.port(),
                            "accepted": s.accepted,
                            "active": s.active,
                            "bytes_up": s.bytes_up,
                            "bytes_down": s.bytes_down,
                            "errors": s.errors,
                        })),
                )
            }
            None => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({ "running": false })),
            ),
        }
    });

    // --- System proxy ---
    let sys_q = sys_proxy.clone();
    router.register("system_proxy.query", move |req| {
        let q = query_system_proxy().unwrap_or_default();
        let auto = sys_q.lock().map(|g| g.is_active()).unwrap_or(false);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "enabled": q.enabled,
                    "server": q.server,
                    "bypass": q.bypass,
                    "auto_managed": auto,
                }),
            ),
        )
    });
    let sys_set = sys_proxy.clone();
    router.register("system_proxy.set", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let enabled = payload
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let server = payload
            .get("server")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let bypass = payload
            .get("bypass")
            .and_then(|v| v.as_str())
            .unwrap_or("localhost;127.*;<local>")
            .to_string();
        let settings = SystemProxySettings {
            enabled,
            server,
            bypass,
        };
        match apply_system_proxy(&settings) {
            Ok(saved) => {
                if let Ok(mut g) = sys_set.lock() {
                    // mark external apply
                    let _ = g;
                    let _ = saved;
                }
                Ok(
                    IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                        .with_payload(serde_json::json!({ "ok": true })),
                )
            }
            Err(e) => err_resp(req, "failed_precondition", e.to_string(), 500),
        }
    });
    let sys_dis = sys_proxy.clone();
    router.register("system_proxy.disable", move |req| {
        if let Ok(mut g) = sys_dis.lock() {
            let _ = g.disable();
        } else {
            let _ = disable_system_proxy();
        }
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "disabled": true })),
        )
    });

    // --- Process live resolve ---
    router.register("process.resolve", move |req| {
        let pid = req
            .payload
            .as_ref()
            .and_then(|p| p.get("pid"))
            .and_then(|v| v.as_u64())
            .ok_or(RouteError::InvalidInput("payload.pid required"))? as u32;
        match netpilot_process::resolve_pid_live(pid) {
            Some(snap) => Ok(IpcEnvelope::ok_response(
                req.request_id.clone(),
                req.operation.clone(),
            )
            .with_payload(serde_json::json!({
                "pid": snap.pid,
                "exe_name": snap.identity.exe_name,
                "exe_path": snap.identity.exe_path,
                "uid_hash": snap.identity.uid_hash,
            }))),
            None => err_resp(req, "not_found", format!("pid {pid} not resolved"), 404),
        }
    });
    router.register("process.list", move |req| {
        let items: Vec<serde_json::Value> = netpilot_process::list_processes_live()
            .into_iter()
            .take(500)
            .map(|s| {
                serde_json::json!({
                    "pid": s.pid,
                    "exe_name": s.identity.exe_name,
                    "exe_path": s.identity.exe_path,
                })
            })
            .collect();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "processes": items, "count": items.len() })),
        )
    });

    let engine_routes = engine.clone();

    let engine_pump = engine.clone();
    router.register("tunnel.pump", move |req| {
        let timeout_ms = req
            .payload
            .as_ref()
            .and_then(|p| p.get("timeout_ms"))
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as u32;
        let mut g = engine_pump
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.pump_and_relay_once(timeout_ms) {
            Ok(r) => {
                let rs = g.relay_stats();
                Ok(
                    IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                        .with_payload(serde_json::json!({
                            "packet_in": r.packet_in,
                            "replies_out": r.replies_out,
                            "dialed": r.dialed,
                            "dial_error": r.dial_error,
                            "relay_error": r.relay_error,
                            "bytes_up": r.bytes_up,
                            "bytes_down": r.bytes_down,
                            "event_kind": r.event_kind,
                            "relay_flows": g.relay_flow_count(),
                            "relay_stats": {
                                "flows": rs.flows,
                                "dial_ok": rs.dial_ok,
                                "dial_fail": rs.dial_fail,
                                "bytes_up": rs.bytes_up,
                                "bytes_down": rs.bytes_down,
                                "pumps": rs.pumps,
                            }
                        })),
                )
            }
            Err(e) => err_resp(req, "failed_precondition", e.to_string(), 500),
        }
    });

    let engine_inj = engine.clone();
    router.register("tunnel.inject", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        // Accept hex-encoded packet for tests
        let hex = payload
            .get("hex")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.hex required"))?;
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        let h = hex.trim();
        let mut i = 0;
        while i + 1 < h.len() {
            let b = u8::from_str_radix(&h[i..i + 2], 16)
                .map_err(|_| RouteError::InvalidInput("bad hex"))?;
            bytes.push(b);
            i += 2;
        }
        let mut g = engine_inj
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.inject_packet_and_relay(&bytes) {
            Ok(r) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({
                        "packet_in": r.packet_in,
                        "replies_out": r.replies_out,
                        "dialed": r.dialed,
                        "dial_error": r.dial_error,
                        "event_kind": r.event_kind,
                        "bytes_up": r.bytes_up,
                    })),
            ),
            Err(e) => err_resp(req, "failed_precondition", e.to_string(), 500),
        }
    });

    router.register("routes.inject", move |req| {
        let payload = req.payload.as_ref();
        let tun_gw = payload
            .and_then(|p| p.get("tun_gateway"))
            .and_then(|v| v.as_str())
            .unwrap_or("10.0.0.1");
        let tun_luid = payload
            .and_then(|p| p.get("tun_luid"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let proxy = payload
            .and_then(|p| p.get("proxy_server"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok());
        let phys_gw = payload
            .and_then(|p| p.get("physical_gateway"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok());
        let phys_luid = payload
            .and_then(|p| p.get("physical_luid"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let tun_gateway: std::net::Ipv4Addr = tun_gw
            .parse()
            .map_err(|_| RouteError::InvalidInput("bad tun_gateway"))?;
        let mut g = engine_routes
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        g.set_physical_gateway(phys_gw, phys_luid);
        match g.inject_routes(tun_gateway, tun_luid, proxy) {
            Ok(n) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({
                        "applied": n,
                        "planned": g.route_plan_len(),
                    })),
            ),
            Err(e) => err_resp(req, "failed_precondition", e.to_string(), 500),
        }
    });

    let engine_rr = engine.clone();
    router.register("routes.rollback", move |req| {
        let mut g = engine_rr
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.rollback_routes() {
            Ok(n) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({ "removed": n })),
            ),
            Err(e) => err_resp(req, "internal", e.to_string(), 500),
        }
    });

    let engine_ns = engine.clone();
    router.register("netstack.stats", move |req| {
        let g = engine_ns
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        let (pin, pout, syns, conns) = g.netstack_stats();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "packets_in": pin,
                    "packets_out": pout,
                    "syns": syns,
                    "connections": conns,
                }),
            ),
        )
    });

    let engine_real = engine.clone();
    router.register("reality.fingerprint", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let sni = payload
            .get("server_name")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("server_name"))?;
        let fp = payload
            .get("fingerprint")
            .and_then(|v| v.as_str())
            .unwrap_or("chrome");
        let g = engine_real
            .lock()
            .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
        match g.reality_probe(sni, fp) {
            Ok((digest, hello_len)) => Ok(IpcEnvelope::ok_response(
                req.request_id.clone(),
                req.operation.clone(),
            )
            .with_payload(serde_json::json!({
                "digest": digest,
                "client_hello_len": hello_len,
                "fingerprint": fp,
                "server_name": sni,
            }))),
            Err(e) => err_resp(req, "invalid_argument", e.to_string(), 400),
        }
    });

    let cm_list = conn_mgr.clone();
    router.register("connections.list", move |req| {
        let g = cm_list
            .lock()
            .map_err(|_| RouteError::Internal("conn lock poisoned"))?;
        let items: Vec<serde_json::Value> = g
            .list()
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "destination": m.destination,
                    "network": m.network,
                    "outbound": m.outbound,
                    "process": m.process_name,
                    "rule": m.rule_summary,
                })
            })
            .collect();
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "items": items })),
        )
    });

    let cfg_load = engine.clone();
    router.register("config.load", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let text = payload
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.text required"))?;
        let format = payload
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");
        let fmt = match format {
            "yaml" | "yml" => netpilot_config::ConfigFormat::Yaml,
            _ => netpilot_config::ConfigFormat::Json,
        };
        match netpilot_config::load_from_str(text, fmt) {
            Ok(doc) => match doc.load_pipeline() {
                Ok(norm) => {
                    let mut g = cfg_load
                        .lock()
                        .map_err(|_| RouteError::Internal("engine lock poisoned"))?;
                    for p in norm.proxies {
                        g.add_profile(p);
                    }
                    Ok(
                        IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                            .with_payload(serde_json::json!({
                                "profiles": g.profiles().len(),
                                "groups": norm.groups.len(),
                                "ok": true,
                            })),
                    )
                }
                Err(e) => err_resp(req, "invalid_argument", e.to_string(), 400),
            },
            Err(e) => err_resp(req, "invalid_argument", e.to_string(), 400),
        }
    });

    router.register("dns.resolve", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let name = payload
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("name"))?;
        // SystemResolver probe is availability; for resolve use std ToSocketAddrs
        let addrs: Vec<String> = format!("{name}:0")
            .to_socket_addrs()
            .map(|i| {
                i.filter_map(|a| match a {
                    std::net::SocketAddr::V4(v) => Some(v.ip().to_string()),
                    std::net::SocketAddr::V6(v) => Some(v.ip().to_string()),
                })
                .collect()
            })
            .unwrap_or_default();
        let _ = SystemResolver::probe();
        let _ = DnsQuery::a(name);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone()).with_payload(
                serde_json::json!({
                    "name": name,
                    "addresses": addrs,
                }),
            ),
        )
    });

    let engine_bg = engine.clone();
    let pump_stop_start = pump_stop.clone();
    router.register("tunnel.pump_loop_start", move |req| {
        pump_stop_start.store(false, Ordering::SeqCst);
        let eng = engine_bg.clone();
        let stop = pump_stop_start.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                if let Ok(mut g) = eng.lock() {
                    let _ = g.pump_and_relay_once(100);
                } else {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "running": true })),
        )
    });

    let pump_stop_stop = pump_stop.clone();
    router.register("tunnel.pump_loop_stop", move |req| {
        pump_stop_stop.store(true, Ordering::SeqCst);
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(serde_json::json!({ "running": false })),
        )
    });

    router.register("outbound.tcp_probe", move |req| {
        let payload = req
            .payload
            .as_ref()
            .ok_or(RouteError::InvalidInput("missing payload"))?;
        let host = payload
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or(RouteError::InvalidInput("payload.host required"))?;
        let port = payload
            .get("port")
            .and_then(|v| v.as_u64())
            .ok_or(RouteError::InvalidInput("payload.port required"))? as u16;
        let timeout_ms = payload
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(3000);
        let dial_req = TcpDialRequest::new(host, port)
            .with_timeout(Duration::from_millis(timeout_ms.max(100)));
        match dial_tcp(&dial_req) {
            Ok(r) => Ok(
                IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                    .with_payload(serde_json::json!({
                        "peer": r.peer,
                        "local": r.local,
                        "elapsed_ms": r.elapsed_ms,
                        "ok": true,
                    })),
            ),
            Err(e) => err_resp(req, "unavailable", e.to_string(), 503),
        }
    });

    router.register("tun.wintun_probe", move |req| {
        let mut session = WintunSession::new(Default::default(), WintunDllPath::BesideExecutable);
        let load = session.load_library();
        let state = format!("{:?}", session.state());
        let mut native = serde_json::json!({
            "attempted": true,
            "session_state": state,
            "load_ok": load.is_ok(),
        });
        #[cfg(windows)]
        {
            match netpilot_os_wintun::load_first_available() {
                Ok(lib) => {
                    native["dll_path"] = serde_json::json!(lib.path().display().to_string());
                    native["exports"] = serde_json::json!(lib.probe_exports());
                    native["dll_loaded"] = serde_json::json!(true);
                }
                Err(e) => {
                    native["dll_loaded"] = serde_json::json!(false);
                    native["dll_error"] = serde_json::json!(e.to_string());
                }
            }
        }
        #[cfg(not(windows))]
        {
            native["dll_loaded"] = serde_json::json!(false);
            native["dll_error"] = serde_json::json!("unsupported platform");
        }
        let _ = load;
        Ok(
            IpcEnvelope::ok_response(req.request_id.clone(), req.operation.clone())
                .with_payload(native),
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
