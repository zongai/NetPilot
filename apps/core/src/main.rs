//! NetPilot Core — resident process with named-pipe IPC (Windows).
//!
//! Env:
//! - `NETPILOT_SMOKE_ONLY=1` — run feature smoke then exit (CI / quick check)
//! - `NETPILOT_IDLE_SECS=N` — non-Windows idle deadline (default 3 in debug tests)

mod service;

use netpilot_core_lib::CoreRuntime;
use netpilot_dns::{DnsCache, DnsPolicy, DnsResolver, FakeIpAllocator, MockTransport};
use netpilot_os_pipe::bare_name;
use netpilot_subscription::{
    run_subscription_pipeline, FilterRule, RenameRule, SubscriptionProfile,
};
use netpilot_tun::{MockTunProvider, TunConfig, TunProvider, WintunFeasibility};
use service::{run_idle_service, run_pipe_service, ServiceControl};
use std::time::Duration;

fn feature_smoke() {
    eprintln!("netpilot-core {}", env!("CARGO_PKG_VERSION"));
    eprintln!("features: runtime,proxy,rules,dns,tun,process,diagnostics,subscription,tls,ipc");
    eprintln!("wintun: {:?}", WintunFeasibility::evaluate().status);
    eprintln!("ipc pipe: \\\\.\\pipe\\{}", bare_name(netpilot_ipc::DEFAULT_PIPE_NAME));

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

fn main() {
    feature_smoke();

    let smoke_only = std::env::var("NETPILOT_SMOKE_ONLY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut runtime = CoreRuntime::new();
    eprintln!("netpilot-core state={}", runtime.state().as_str());

    if let Err(err) = runtime.start() {
        eprintln!("start failed: {err}");
        std::process::exit(1);
    }
    eprintln!("netpilot-core state={}", runtime.state().as_str());

    if smoke_only {
        if let Err(err) = runtime.shutdown() {
            eprintln!("shutdown failed: {err}");
            std::process::exit(1);
        }
        eprintln!("netpilot-core state={} (smoke only)", runtime.state().as_str());
        return;
    }

    let control = ServiceControl::new();

    #[cfg(windows)]
    {
        if let Err(e) = run_pipe_service(&runtime, control.clone()) {
            eprintln!("netpilot-core: service error: {e}");
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
        run_idle_service(&runtime, control.clone(), idle.or(Some(Duration::from_secs(2))));
    }

    if let Err(err) = runtime.shutdown() {
        eprintln!("shutdown failed: {err}");
        std::process::exit(1);
    }
    eprintln!("netpilot-core state={}", runtime.state().as_str());
}
