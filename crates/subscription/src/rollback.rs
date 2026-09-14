//! Update failure rollback (NP-128).

use crate::cache::{CacheEntry, SubscriptionCache};

#[derive(Debug, Default)]
pub struct RollbackGuard {
    snapshot: Option<(String, CacheEntry)>,
}

impl RollbackGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&mut self, id: &str, cache: &SubscriptionCache) {
        self.snapshot = cache.get(id).map(|e| (id.to_string(), e.clone()));
    }

    pub fn commit(&mut self) {
        self.snapshot = None;
    }

    pub fn rollback(&mut self, cache: &mut SubscriptionCache) -> bool {
        if let Some((id, entry)) = self.snapshot.take() {
            cache.put(&id, entry.body, entry.etag);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_previous() {
        let mut cache = SubscriptionCache::new();
        cache.put("s", "old".into(), None);
        let mut g = RollbackGuard::new();
        g.begin("s", &cache);
        cache.put("s", "new-bad".into(), None);
        assert!(g.rollback(&mut cache));
        assert_eq!(cache.get("s").unwrap().body, "old");
    }
}
