//! Local subscription cache (NP-126).

use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub body: String,
    pub etag: Option<String>,
    pub stored_at: SystemTime,
    pub generation: u64,
}

#[derive(Debug, Default)]
pub struct SubscriptionCache {
    by_id: HashMap<String, CacheEntry>,
    next_gen: u64,
}

impl SubscriptionCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&mut self, id: &str, body: String, etag: Option<String>) -> u64 {
        self.next_gen = self.next_gen.saturating_add(1);
        let gen = self.next_gen;
        self.by_id.insert(
            id.to_string(),
            CacheEntry {
                body,
                etag,
                stored_at: SystemTime::now(),
                generation: gen,
            },
        );
        gen
    }

    pub fn get(&self, id: &str) -> Option<&CacheEntry> {
        self.by_id.get(id)
    }

    pub fn invalidate(&mut self, id: &str) {
        self.by_id.remove(id);
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get() {
        let mut c = SubscriptionCache::new();
        c.put("s1", "x".into(), Some("e".into()));
        assert_eq!(c.get("s1").unwrap().body, "x");
    }
}
