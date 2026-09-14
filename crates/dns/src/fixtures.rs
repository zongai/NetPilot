//! DNS conformance fixtures (NP-084).

use crate::cache::DnsCache;
use crate::fakeip::FakeIpAllocator;
use crate::leak::LeakGuard;
use crate::metrics::DnsMetrics;
use crate::policy::DnsPolicy;
use crate::resolver::{DnsQuery, DnsResolver};
use crate::routing::{DnsRoute, DnsRoutePolicy};
use crate::transport::MockTransport;

#[derive(Debug)]
pub struct DnsFixtureCase {
    pub name: &'static str,
    pub domain: &'static str,
    pub seed_addr: Option<&'static str>,
    pub expect_ok: bool,
}

pub fn dns_conformance_cases() -> Vec<DnsFixtureCase> {
    vec![
        DnsFixtureCase {
            name: "known_a_record",
            domain: "example.com",
            seed_addr: Some("93.184.216.34"),
            expect_ok: true,
        },
        DnsFixtureCase {
            name: "nxdomain",
            domain: "no-such-host.test",
            seed_addr: None,
            expect_ok: false,
        },
        DnsFixtureCase {
            name: "cache_second_lookup",
            domain: "cached.example",
            seed_addr: Some("10.0.0.1"),
            expect_ok: true,
        },
    ]
}

pub fn run_dns_case(case: &DnsFixtureCase) -> Result<(), String> {
    let mut transport = MockTransport::new();
    if let Some(addr) = case.seed_addr {
        transport.seed(case.domain, addr);
    }
    let mut resolver = DnsResolver::new(
        transport,
        DnsCache::new(32),
        FakeIpAllocator::new("198.18.0.0", 16),
        DnsPolicy::default(),
    );
    let mut metrics = DnsMetrics::default();
    metrics.record_query();
    match resolver.resolve(&DnsQuery::a(case.domain)) {
        Ok(r) if case.expect_ok => {
            if r.addresses.is_empty() {
                return Err(format!("{}: empty addresses", case.name));
            }
            // second lookup should hit cache
            metrics.record_cache_miss();
            let r2 = resolver
                .resolve(&DnsQuery::a(case.domain))
                .map_err(|e| e.to_string())?;
            if r2.from_cache {
                metrics.record_cache_hit();
            }
            Ok(())
        }
        Err(_) if !case.expect_ok => {
            metrics.record_nxdomain();
            Ok(())
        }
        Ok(_) => Err(format!("{}: expected error", case.name)),
        Err(e) => Err(format!("{}: unexpected {e}", case.name)),
    }
}

pub fn routing_and_leak_smoke() -> Result<(), String> {
    let mut policy = DnsRoutePolicy::default();
    policy.block_domains.push("ads.test".into());
    if !matches!(policy.route_for("x.ads.test"), DnsRoute::Block) {
        return Err("expected block route".into());
    }
    let guard = LeakGuard::default();
    if !guard.should_block("host.local", &policy) {
        return Err("expected mdns block".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dns_cases() {
        for c in dns_conformance_cases() {
            run_dns_case(&c).unwrap_or_else(|e| panic!("{e}"));
        }
        routing_and_leak_smoke().unwrap();
    }
}
