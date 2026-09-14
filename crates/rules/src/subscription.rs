//! Remote subscription lifecycle (NP-092).

use std::time::{Duration, Instant};

use crate::ruleset::RuleSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionState {
    Idle,
    Fetching,
    Ready,
    Stale,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub url: String,
    pub interval: Duration,
    pub state: SubscriptionState,
    pub last_success: Option<Instant>,
    pub last_error: Option<String>,
    pub etag: Option<String>,
}

impl Subscription {
    pub fn new(id: impl Into<String>, url: impl Into<String>, interval: Duration) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            interval,
            state: SubscriptionState::Idle,
            last_success: None,
            last_error: None,
            etag: None,
        }
    }

    pub fn begin_fetch(&mut self) {
        self.state = SubscriptionState::Fetching;
        self.last_error = None;
    }

    pub fn mark_ready(&mut self, etag: Option<String>) {
        self.state = SubscriptionState::Ready;
        self.last_success = Some(Instant::now());
        self.etag = etag;
        self.last_error = None;
    }

    pub fn mark_failed(&mut self, err: impl Into<String>) {
        self.state = SubscriptionState::Failed;
        self.last_error = Some(err.into());
    }

    pub fn mark_stale_if_needed(&mut self) {
        if let Some(t) = self.last_success {
            if t.elapsed() > self.interval {
                self.state = SubscriptionState::Stale;
            }
        }
    }
}

/// Holds active subscriptions and last applied payload (text only; no network).
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    items: Vec<Subscription>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, sub: Subscription) {
        self.items.push(sub);
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Subscription> {
        self.items.iter_mut().find(|s| s.id == id)
    }

    pub fn items(&self) -> &[Subscription] {
        &self.items
    }

    /// Apply fetched body into a RuleSet (caller supplies id/name).
    pub fn materialize(
        body: &str,
        ruleset_id: &str,
        name: &str,
    ) -> Result<RuleSet, crate::loader::LoadError> {
        crate::loader::load_ruleset_from_str(ruleset_id, name, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle() {
        let mut s = Subscription::new("s1", "https://example.com/rules", Duration::from_secs(3600));
        s.begin_fetch();
        assert_eq!(s.state, SubscriptionState::Fetching);
        s.mark_ready(Some("etag1".into()));
        assert_eq!(s.state, SubscriptionState::Ready);
    }
}
