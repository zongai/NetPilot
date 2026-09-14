//! RuleSet cache and freshness (NP-093).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::ruleset::RuleSet;

#[derive(Debug, Clone)]
struct CacheEntry {
    set: RuleSet,
    stored_at: Instant,
    ttl: Duration,
}

#[derive(Debug, Default)]
pub struct RuleSetCache {
    map: HashMap<String, CacheEntry>,
}

impl RuleSetCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&mut self, set: RuleSet, ttl: Duration) {
        let id = set.id.clone();
        self.map.insert(
            id,
            CacheEntry {
                set,
                stored_at: Instant::now(),
                ttl,
            },
        );
    }

    pub fn get(&self, id: &str) -> Option<&RuleSet> {
        let e = self.map.get(id)?;
        if e.stored_at.elapsed() > e.ttl {
            return None;
        }
        Some(&e.set)
    }

    pub fn is_fresh(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    pub fn invalidate(&mut self, id: &str) {
        self.map.remove(id);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_ruleset_from_str;

    #[test]
    fn ttl_freshness() {
        let set = load_ruleset_from_str("c1", "C", "MATCH,DIRECT\n").unwrap();
        let mut cache = RuleSetCache::new();
        cache.put(set, Duration::from_secs(60));
        assert!(cache.is_fresh("c1"));
        cache.invalidate("c1");
        assert!(!cache.is_fresh("c1"));
    }
}
