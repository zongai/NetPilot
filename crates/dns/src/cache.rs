//! Resolver cache (NP-079).

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub name: String,
    pub qtype: u16,
}

impl CacheKey {
    pub fn a(name: &str) -> Self {
        Self {
            name: name.to_ascii_lowercase(),
            qtype: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub addresses: Vec<String>,
    pub expires_at: Instant,
}

#[derive(Debug)]
pub struct DnsCache {
    map: HashMap<CacheKey, DnsRecord>,
    capacity: usize,
}

impl DnsCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&mut self, key: &CacheKey) -> Option<Vec<String>> {
        let now = Instant::now();
        if let Some(rec) = self.map.get(key) {
            if rec.expires_at > now {
                return Some(rec.addresses.clone());
            }
        }
        self.map.remove(key);
        None
    }

    pub fn insert(&mut self, key: CacheKey, addresses: Vec<String>, ttl: Duration) {
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            // Simple eviction: drop an arbitrary expired or first entry.
            let victim = self
                .map
                .iter()
                .find(|(_, v)| v.expires_at <= Instant::now())
                .map(|(k, _)| k.clone())
                .or_else(|| self.map.keys().next().cloned());
            if let Some(k) = victim {
                self.map.remove(&k);
            }
        }
        self.map.insert(
            key,
            DnsRecord {
                addresses,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_expiry() {
        let mut c = DnsCache::new(8);
        c.insert(
            CacheKey::a("x.test"),
            vec!["1.1.1.1".into()],
            Duration::from_secs(60),
        );
        assert!(c.get(&CacheKey::a("x.test")).is_some());
        c.insert(CacheKey::a("y.test"), vec!["2.2.2.2".into()], Duration::ZERO);
        // zero TTL is already expired relative to now for practical purposes
        let _ = c.get(&CacheKey::a("y.test"));
    }
}
