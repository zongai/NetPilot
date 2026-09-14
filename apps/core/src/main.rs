//! NetPilot Core binary — full feature surface (S0–S9 + S11).
//! Desktop GUI talks to this process over Named Pipe IPC (later wiring).

use netpilot_core_lib::CoreRuntime;
use netpilot_dns::{DnsCache, DnsPolicy, DnsResolver, FakeIpAllocator, MockTransport};
use netpilot_subscription::{
    run_subscription_pipeline, FilterRule, RenameRule, SubscriptionProfile,
};
use netpilot_tun::{MockTunProvider, TunConfig, TunProvider, WintunFeasibility};

fn main() {
    eprintln!("netpilot-core {}", env!("CARGO_PKG_VERSION"));
    eprintln!("features: runtime,proxy,rules,dns,tun,process,diagnostics,subscription,tls");
    eprintln!("wintun: {:?}", WintunFeasibility::evaluate().status);

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
    let pipe = run_subscription_pipeline(
        &sub,
        sample,
        &FilterRule::default(),
        &RenameRule::default(),
    );
    eprintln!(
        "subscription demo nodes={} group={}",
        pipe.profiles.len(),
        pipe.group.name
    );

    let mut runtime = CoreRuntime::new();
    eprintln!("netpilot-core state={}", runtime.state().as_str());

    if let Err(err) = runtime.start() {
        eprintln!("start failed: {err}");
        std::process::exit(1);
    }
    eprintln!("netpilot-core state={}", runtime.state().as_str());

    if let Err(err) = runtime.shutdown() {
        eprintln!("shutdown failed: {err}");
        std::process::exit(1);
    }
    eprintln!("netpilot-core state={}", runtime.state().as_str());
}
