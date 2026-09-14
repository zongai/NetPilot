//! Subscription manager (NP-121).

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::cache::SubscriptionCache;
use crate::fetcher::{FetchError, FetchRequest, SubscriptionFetcher};
use crate::profile::{SubscriptionProfile, SubscriptionState};
use crate::rollback::RollbackGuard;

#[derive(Debug)]
pub enum SubscriptionError {
    Profile(&'static str),
    Fetch(FetchError),
    NotFound,
}

impl std::fmt::Display for SubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Profile(m) => write!(f, "Profile: {m}"),
            Self::Fetch(e) => write!(f, "Fetch: {e}"),
            Self::NotFound => write!(f, "NotFound"),
        }
    }
}

impl std::error::Error for SubscriptionError {}

#[derive(Debug, Default)]
pub struct SubscriptionManager {
    profiles: HashMap<String, SubscriptionProfile>,
    pub cache: SubscriptionCache,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, profile: SubscriptionProfile) -> Result<(), SubscriptionError> {
        profile.validate().map_err(SubscriptionError::Profile)?;
        self.profiles.insert(profile.id.clone(), profile);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&SubscriptionProfile> {
        self.profiles.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut SubscriptionProfile> {
        self.profiles.get_mut(id)
    }

    pub fn list(&self) -> Vec<&SubscriptionProfile> {
        self.profiles.values().collect()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.cache.invalidate(id);
        self.profiles.remove(id).is_some()
    }

    /// Fetch and store body; on failure roll back cache for this id.
    pub fn update_one(
        &mut self,
        id: &str,
        fetcher: &mut dyn SubscriptionFetcher,
        timeout: Duration,
    ) -> Result<String, SubscriptionError> {
        let profile = self
            .profiles
            .get_mut(id)
            .ok_or(SubscriptionError::NotFound)?;
        profile.validate().map_err(SubscriptionError::Profile)?;
        profile.mark_updating();
        let req = FetchRequest::from_profile(profile, timeout);
        let mut guard = RollbackGuard::new();
        guard.begin(id, &self.cache);

        match fetcher.fetch(&req) {
            Ok(resp) => {
                self.cache.put(id, resp.body.clone(), resp.etag.clone());
                if let Some(p) = self.profiles.get_mut(id) {
                    p.mark_ready(resp.etag, resp.last_modified);
                }
                guard.commit();
                Ok(resp.body)
            }
            Err(FetchError::NotModified) => {
                if let Some(p) = self.profiles.get_mut(id) {
                    p.state = SubscriptionState::Ready;
                    p.last_success = Some(SystemTime::now());
                }
                guard.commit();
                self.cache
                    .get(id)
                    .map(|e| e.body.clone())
                    .ok_or(SubscriptionError::Fetch(FetchError::NotModified))
            }
            Err(e) => {
                guard.rollback(&mut self.cache);
                if let Some(p) = self.profiles.get_mut(id) {
                    p.mark_failed(e.to_string());
                }
                Err(SubscriptionError::Fetch(e))
            }
        }
    }

    pub fn update_due(
        &mut self,
        fetcher: &mut dyn SubscriptionFetcher,
        timeout: Duration,
        now: SystemTime,
    ) -> Vec<(String, Result<String, SubscriptionError>)> {
        let ids: Vec<String> = self
            .profiles
            .values()
            .filter(|p| p.is_due(now))
            .map(|p| p.id.clone())
            .collect();
        ids.into_iter()
            .map(|id| {
                let r = self.update_one(&id, fetcher, timeout);
                (id, r)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::MockFetcher;

    #[test]
    fn update_and_fail_keeps_old() {
        let mut mgr = SubscriptionManager::new();
        mgr.upsert(SubscriptionProfile::new("s", "S", "https://x/sub"))
            .unwrap();
        let mut f = MockFetcher::new();
        f.seed("https://x/sub", "v1", Some("e1"));
        assert_eq!(
            mgr.update_one("s", &mut f, Duration::from_secs(5)).unwrap(),
            "v1"
        );
        f.force_status = Some(500);
        assert!(mgr.update_one("s", &mut f, Duration::from_secs(5)).is_err());
        assert_eq!(mgr.cache.get("s").unwrap().body, "v1");
    }
}
