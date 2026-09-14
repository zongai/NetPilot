//! DNS resolver abstraction (NP-073).

use std::time::Duration;

use crate::cache::{CacheKey, DnsCache};
use crate::fakeip::FakeIpAllocator;
use crate::policy::{DnsPolicy, ResolveMode};
use crate::transport::{DnsTransport, TransportKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQuery {
    pub name: String,
    pub qtype: u16,
}

impl DnsQuery {
    pub fn a(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            qtype: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsResponse {
    pub name: String,
    pub addresses: Vec<String>,
    pub ttl_secs: u32,
    pub from_cache: bool,
    pub transport: TransportKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsError {
    NxDomain(String),
    Transport(String),
    InvalidInput(&'static str),
    Exhausted(&'static str),
}

impl std::fmt::Display for DnsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NxDomain(n) => write!(f, "NxDomain: {n}"),
            Self::Transport(m) => write!(f, "Transport: {m}"),
            Self::InvalidInput(m) => write!(f, "InvalidInput: {m}"),
            Self::Exhausted(m) => write!(f, "Exhausted: {m}"),
        }
    }
}

impl std::error::Error for DnsError {}

pub struct DnsResolver<T: DnsTransport> {
    transport: T,
    cache: DnsCache,
    fakeip: FakeIpAllocator,
    policy: DnsPolicy,
}

impl<T: DnsTransport> DnsResolver<T> {
    pub fn new(transport: T, cache: DnsCache, fakeip: FakeIpAllocator, policy: DnsPolicy) -> Self {
        Self {
            transport,
            cache,
            fakeip,
            policy,
        }
    }

    pub fn policy(&self) -> &DnsPolicy {
        &self.policy
    }

    pub fn resolve(&mut self, query: &DnsQuery) -> Result<DnsResponse, DnsError> {
        if query.name.trim().is_empty() {
            return Err(DnsError::InvalidInput("empty name"));
        }
        let key = CacheKey {
            name: query.name.to_ascii_lowercase(),
            qtype: query.qtype,
        };

        if self.policy.cache_enabled {
            if let Some(addrs) = self.cache.get(&key) {
                return Ok(DnsResponse {
                    name: query.name.clone(),
                    addresses: addrs,
                    ttl_secs: self.policy.default_ttl_secs,
                    from_cache: true,
                    transport: self.transport.kind(),
                });
            }
        }

        if self.policy.mode == ResolveMode::FakeIp {
            let ip = self.fakeip.allocate(&query.name)?;
            return Ok(DnsResponse {
                name: query.name.clone(),
                addresses: vec![ip],
                ttl_secs: self.policy.default_ttl_secs,
                from_cache: false,
                transport: TransportKind::Mock,
            });
        }

        let mut resp = self.transport.query(query)?;
        if self.policy.cache_enabled {
            self.cache.insert(
                key,
                resp.addresses.clone(),
                Duration::from_secs(resp.ttl_secs.max(1) as u64),
            );
        }
        resp.from_cache = false;
        Ok(resp)
    }

    pub fn allocate_fakeip(&mut self, domain: &str) -> Result<String, DnsError> {
        self.fakeip.allocate(domain)
    }

    pub fn lookup_fakeip(&self, ip: &str) -> Option<String> {
        self.fakeip.lookup_domain(ip).map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;

    #[test]
    fn empty_name_rejected() {
        let mut r = DnsResolver::new(
            MockTransport::new(),
            DnsCache::new(8),
            FakeIpAllocator::new("198.18.0.0", 16),
            DnsPolicy::default(),
        );
        assert!(matches!(
            r.resolve(&DnsQuery::a("")),
            Err(DnsError::InvalidInput(_))
        ));
    }
}
