//! NetPilot Core — resident process with named-pipe IPC (Windows).
//!
//! Env:
//! - `NETPILOT_SMOKE_ONLY=1` — run feature smoke then exit (CI / quick check)
//! - `NETPILOT_IDLE_SECS=N` — non-Windows idle deadline (default 3 in debug tests)
//! - `NETPILOT_DATA_DIR` — override runtime data root (NP-151)
//! - `NETPILOT_LOG_LEVEL` — error|warn|info|debug|trace (NP-152)

mod service;

use netpilot_config::RuntimePaths;
use netpilot_core_lib::{
    log_line, set_max_level, CoreHealth, CoreRuntime, InstanceLock, LogLevel, ShutdownCoordinator,
    ShutdownReason,
};
use netpilot_dns::{DnsCache, DnsPolicy, DnsResolver, FakeIpAllocator, MockTransport};
use netpilot_os_pipe::bare_name;
use netpilot_subscription::{
    run_subscription_pipeline, FilterRule, RenameRule, SubscriptionProfile,
};
use netpilot_tun::{MockTunProvider, TunConfig, TunProvider, WintunFeasibility};
#[cfg(not(windows))]
use service::run_idle_service;
#[cfg(windows)]
use service::run_pipe_service;
use service::ServiceControl;
#[cfg(not(windows))]
use std::time::Duration;
use std::time::Instant;

fn feature_smoke() {
    eprintln!("netpilot-core {}", env!("CARGO_PKG_VERSION"));
    eprintln!("features: runtime,proxy,rules,dns,tun,process,diagnostics,subscription,tls,ipc");
    eprintln!("wintun: {:?}", WintunFeasibility::evaluate().status);
    eprintln!(
        "ipc pipe: \\\\.\\pipe\\{}",
        bare_name(netpilot_ipc::DEFAULT_PIPE_NAME)
    );

    let _compat = netpilot_protocol_common::compatibility_matrix().len();
    let _tls = netpilot_transport_tls::transport_id();

    let _resolver = DnsResolver::new(
        MockTransport::new(),
        DnsCache::new(32),
        FakeIpAllocator::new("198.18.0.0", 16),
        DnsPolicy::default(),
    );

    let mut tun = MockTunProvider::new();
    if let Ok(dev) = tun.open(TunConfig::default()) {
        eprintln!("tun mock device opened: {}", dev.name());
    }

    let sub = SubscriptionProfile::new("builtin", "demo", "https://example.com/sub");
    let sample = "trojan://pass@example.com:443?security=tls#demo-node\n";
    let pipe =
        run_subscription_pipeline(&sub, sample, &FilterRule::default(), &RenameRule::default());
    eprintln!(
        "subscription demo nodes={} group={}",
        pipe.profiles.len(),
        pipe.group.name
    );
}

fn init_logging() {
    if let Ok(v) = std::env::var("NETPILOT_LOG_LEVEL") {
        if let Some(level) = LogLevel::parse(&v) {
            set_max_level(level);
        }
    }
}

fn main() {
    init_logging();
    feature_smoke();

    let smoke_only = std::env::var("NETPILOT_SMOKE_ONLY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let paths = RuntimePaths::resolve();
    if let Err(e) = paths.ensure_dirs() {
        eprintln!("data dirs: {e}");
    }
    log_line(
        LogLevel::Info,
        "core",
        &format!("data root={}", paths.root.display()),
    );

    // Single-instance guard (NP-150). Skip in smoke so parallel CI is fine.
    let _instance = if smoke_only {
        None
    } else {
        match InstanceLock::try_acquire(&paths.run_dir) {
            Ok(lock) => {
                log_line(
                    LogLevel::Info,
                    "core",
                    &format!("instance lock pid={}", lock.pid()),
                );
                Some(lock)
            }
            Err(err) => {
                eprintln!("instance lock: {err}");
                std::process::exit(2);
            }
        }
    };

    let started_at = Instant::now();
    let shutdown = ShutdownCoordinator::new();
    let mut runtime = CoreRuntime::new();
    log_line(
        LogLevel::Info,
        "core",
        &format!("state={}", runtime.state().as_str()),
    );

    if let Err(err) = runtime.start() {
        eprintln!("start failed: {err}");
        std::process::exit(1);
    }
    let health = CoreHealth::ready(started_at, cfg!(windows));
    log_line(
        LogLevel::Info,
        "core",
        &format!(
            "state={} health={} ipc_bound={}",
            runtime.state().as_str(),
            health.level.as_str(),
            health.ipc_bound
        ),
    );

    if smoke_only {
        shutdown.request(ShutdownReason::Explicit, std::time::Duration::from_secs(5));
        if let Err(err) = runtime.shutdown() {
            eprintln!("shutdown failed: {err}");
            std::process::exit(1);
        }
        log_line(
            LogLevel::Info,
            "core",
            &format!("state={} (smoke only)", runtime.state().as_str()),
        );
        return;
    }

    let control = ServiceControl::new();

    #[cfg(windows)]
    {
        if let Err(e) = run_pipe_service(&runtime, control.clone()) {
            eprintln!("netpilot-core: service error: {e}");
            shutdown.request(ShutdownReason::Fatal, std::time::Duration::from_secs(10));
            let _ = runtime.shutdown();
            std::process::exit(1);
        }
    }

    #[cfg(not(windows))]
    {
        let idle = std::env::var("NETPILOT_IDLE_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs);
        run_idle_service(
            &runtime,
            control.clone(),
            idle.or(Some(Duration::from_secs(2))),
        );
    }

    shutdown.request(ShutdownReason::Explicit, std::time::Duration::from_secs(15));
    if let Err(err) = runtime.shutdown() {
        eprintln!("shutdown failed: {err}");
        std::process::exit(1);
    }
    log_line(
        LogLevel::Info,
        "core",
        &format!(
            "state={} reason={:?}",
            runtime.state().as_str(),
            shutdown.reason()
        ),
    );
}
