//! Subscription profile model (NP-122).

use std::time::{Duration, SystemTime};

use crate::http::HttpRequestHeaders;
use crate::policy::UpdatePolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionState {
    Idle,
    Updating,
    Ready,
    Stale,
    Failed,
}

#[derive(Debug, Clone)]
pub struct SubscriptionProfile {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub policy: UpdatePolicy,
    pub headers: HttpRequestHeaders,
    pub state: SubscriptionState,
    pub last_success: Option<SystemTime>,
    pub last_error: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl SubscriptionProfile {
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            url: url.into(),
            enabled: true,
            policy: UpdatePolicy::default(),
            headers: HttpRequestHeaders::default(),
            state: SubscriptionState::Idle,
            last_success: None,
            last_error: None,
            etag: None,
            last_modified: None,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() {
            return Err("id required");
        }
        if self.name.trim().is_empty() {
            return Err("name required");
        }
        let u = self.url.trim();
        if !(u.starts_with("https://") || u.starts_with("http://")) {
            return Err("url must be http(s)");
        }
        Ok(())
    }

    pub fn mark_updating(&mut self) {
        self.state = SubscriptionState::Updating;
        self.last_error = None;
    }

    pub fn mark_ready(&mut self, etag: Option<String>, last_modified: Option<String>) {
        self.state = SubscriptionState::Ready;
        self.last_success = Some(SystemTime::now());
        self.etag = etag;
        self.last_modified = last_modified;
        self.last_error = None;
    }

    pub fn mark_failed(&mut self, err: impl Into<String>) {
        self.state = SubscriptionState::Failed;
        self.last_error = Some(err.into());
    }

    pub fn is_due(&self, now: SystemTime) -> bool {
        if !self.enabled {
            return false;
        }
        match self.policy.interval {
            None => false,
            Some(interval) if interval.is_zero() => false,
            Some(interval) => match self.last_success {
                None => true,
                Some(t) => now.duration_since(t).unwrap_or(Duration::ZERO) >= interval,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_https() {
        let p = SubscriptionProfile::new("1", "A", "https://example.com/sub");
        assert!(p.validate().is_ok());
        let bad = SubscriptionProfile::new("1", "A", "ftp://x");
        assert!(bad.validate().is_err());
    }
}
