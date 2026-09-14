//! DNS leak prevention (NP-082).

use crate::routing::{DnsRoute, DnsRoutePolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakRisk {
    /// Query may leave the host via system stack outside tunnel.
    SystemBypass,
    /// Multicast / link-local name resolution.
    LinkLocal,
    /// Allowed by policy.
    None,
}

#[derive(Debug, Clone)]
pub struct LeakGuard {
    /// When true, system route is only allowed for listed domains.
    pub strict_tunnel: bool,
    pub block_mdns: bool,
}

impl Default for LeakGuard {
    fn default() -> Self {
        Self {
            strict_tunnel: true,
            block_mdns: true,
        }
    }
}

impl LeakGuard {
    pub fn assess(&self, name: &str, route: &DnsRoute) -> LeakRisk {
        let n = name.to_ascii_lowercase();
        if self.block_mdns && (n.ends_with(".local") || n == "local") {
            return LeakRisk::LinkLocal;
        }
        if self.strict_tunnel && matches!(route, DnsRoute::System) {
            return LeakRisk::SystemBypass;
        }
        LeakRisk::None
    }

    /// Returns true if the query should be refused to prevent leaks.
    pub fn should_block(&self, name: &str, policy: &DnsRoutePolicy) -> bool {
        let route = policy.route_for(name);
        match self.assess(name, &route) {
            LeakRisk::LinkLocal => true,
            LeakRisk::SystemBypass => {
                // Allow only if explicitly listed as system domain under strict mode.
                let n = name.trim().trim_end_matches('.').to_ascii_lowercase();
                !policy.system_domains.iter().any(|p| {
                    let p = p.trim().trim_end_matches('.').to_ascii_lowercase();
                    n == p
                        || n.ends_with(&format!(".{p}"))
                        || p.strip_prefix('.')
                            .map(|s| n == s || n.ends_with(&format!(".{s}")))
                            .unwrap_or(false)
                })
            }
            LeakRisk::None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_mdns() {
        let g = LeakGuard::default();
        let p = DnsRoutePolicy::default();
        assert!(g.should_block("foo.local", &p));
    }

    #[test]
    fn allows_explicit_system_domain() {
        let g = LeakGuard::default();
        let mut p = DnsRoutePolicy::default();
        p.system_domains.push("corp.internal".into());
        assert!(!g.should_block("corp.internal", &p));
    }
}
