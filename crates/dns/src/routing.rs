//! DNS routing policy (NP-081).

use crate::policy::ResolveMode;
use crate::transport::TransportKind;

/// Where a DNS query should be answered from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRoute {
    /// Use OS system resolver.
    System,
    /// Query via tunnel outbound / configured transport.
    Tunnel(TransportKind),
    /// Answer with Fake-IP only.
    FakeIp,
    /// Block the query (no answer / REFUSED semantics).
    Block,
}

#[derive(Debug, Clone)]
pub struct DnsRoutePolicy {
    pub default_mode: ResolveMode,
    pub preferred_transport: TransportKind,
    /// Domains (exact or suffix with leading '.') forced to system.
    pub system_domains: Vec<String>,
    /// Domains forced to block.
    pub block_domains: Vec<String>,
    /// Domains forced to fake-ip.
    pub fakeip_domains: Vec<String>,
}

impl Default for DnsRoutePolicy {
    fn default() -> Self {
        Self {
            default_mode: ResolveMode::Tunnel,
            preferred_transport: TransportKind::Udp,
            system_domains: vec![],
            block_domains: vec![],
            fakeip_domains: vec![],
        }
    }
}

impl DnsRoutePolicy {
    pub fn route_for(&self, name: &str) -> DnsRoute {
        let n = name.trim().trim_end_matches('.').to_ascii_lowercase();
        if domain_list_match(&self.block_domains, &n) {
            return DnsRoute::Block;
        }
        if domain_list_match(&self.system_domains, &n) {
            return DnsRoute::System;
        }
        if domain_list_match(&self.fakeip_domains, &n) {
            return DnsRoute::FakeIp;
        }
        match self.default_mode {
            ResolveMode::System => DnsRoute::System,
            ResolveMode::Tunnel => DnsRoute::Tunnel(self.preferred_transport),
            ResolveMode::FakeIp => DnsRoute::FakeIp,
        }
    }
}

fn domain_list_match(list: &[String], name: &str) -> bool {
    list.iter().any(|pat| {
        let p = pat.trim().trim_end_matches('.').to_ascii_lowercase();
        if p.is_empty() {
            return false;
        }
        if let Some(suffix) = p.strip_prefix('.') {
            name == suffix || name.ends_with(&format!(".{suffix}"))
        } else {
            name == p || name.ends_with(&format!(".{p}"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_by_list() {
        let mut p = DnsRoutePolicy::default();
        p.block_domains.push("ads.example".into());
        p.system_domains.push(".lan".into());
        assert_eq!(p.route_for("x.ads.example"), DnsRoute::Block);
        assert_eq!(p.route_for("printer.lan"), DnsRoute::System);
        assert!(matches!(p.route_for("google.com"), DnsRoute::Tunnel(_)));
    }
}
