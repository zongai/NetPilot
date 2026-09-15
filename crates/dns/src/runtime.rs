//! DNS runtime façade (NP-229).

use crate::{DnsCache, DnsPolicy, DnsResolver, DnsTransport, FakeIpAllocator, MockTransport};

/// Bundled DNS stack used by Core.
pub struct DnsRuntime<T: DnsTransport> {
    pub resolver: DnsResolver<T>,
}

impl DnsRuntime<MockTransport> {
    pub fn mock_default() -> Self {
        Self {
            resolver: DnsResolver::new(
                MockTransport::new(),
                DnsCache::new(256),
                FakeIpAllocator::new("198.18.0.0", 16),
                DnsPolicy::default(),
            ),
        }
    }
}

impl<T: DnsTransport> DnsRuntime<T> {
    pub fn new(transport: T, cache_cap: usize, policy: DnsPolicy) -> Self {
        Self {
            resolver: DnsResolver::new(
                transport,
                DnsCache::new(cache_cap),
                FakeIpAllocator::new("198.18.0.0", 16),
                policy,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_mock_runtime() {
        let _rt = DnsRuntime::mock_default();
    }
}
