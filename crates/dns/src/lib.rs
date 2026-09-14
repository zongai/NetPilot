//! DNS resolver stack (NP-073…NP-084).
//!
//! Transports are trait-based; unit tests use mock I/O without network sockets.

#![forbid(unsafe_code)]

mod cache;
mod fakeip;
mod fixtures;
mod leak;
mod metrics;
mod policy;
mod resolver;
mod routing;
mod system;
mod transport;

pub use cache::{CacheKey, DnsCache, DnsRecord};
pub use fakeip::{FakeIpAllocator, FakeIpMapping};
pub use fixtures::{dns_conformance_cases, run_dns_case, routing_and_leak_smoke, DnsFixtureCase};
pub use leak::{LeakGuard, LeakRisk};
pub use metrics::DnsMetrics;
pub use policy::{DnsPolicy, ResolveMode};
pub use resolver::{DnsError, DnsQuery, DnsResponse, DnsResolver};
pub use routing::{DnsRoute, DnsRoutePolicy};
pub use system::{SystemResolver, SystemResolverInfo};
pub use transport::{
    DnsTransport, DohTransport, DoTTransport, MockTransport, TcpDnsTransport, TransportKind,
    UdpDnsTransport,
};

pub const CRATE_NAME: &str = "netpilot-dns";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_with_cache_and_fakeip() {
        let mut transport = MockTransport::new();
        transport.seed("example.com", "93.184.216.34");

        let cache = DnsCache::new(64);
        let fakeip = FakeIpAllocator::new("198.18.0.0", 16);
        let policy = DnsPolicy::default();

        let mut resolver = DnsResolver::new(transport, cache, fakeip, policy);
        let r1 = resolver.resolve(&DnsQuery::a("example.com")).unwrap();
        assert!(!r1.addresses.is_empty());
        let r2 = resolver.resolve(&DnsQuery::a("example.com")).unwrap();
        assert!(r2.from_cache);

        let mapped = resolver.allocate_fakeip("example.com").unwrap();
        assert!(mapped.starts_with("198.18."));
        assert_eq!(
            resolver.lookup_fakeip(&mapped).as_deref(),
            Some("example.com")
        );
    }
}
