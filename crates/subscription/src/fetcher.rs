//! HTTP fetcher with timeout / cancel (NP-123) and conditional requests (NP-125).
//! Default builds use an in-memory mock transport (no real sockets in CI).

use std::collections::HashMap;
use std::time::Duration;

use crate::http::HttpRequestHeaders;
use crate::profile::SubscriptionProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    Timeout,
    Cancelled,
    HttpStatus(u16),
    Network(String),
    NotModified,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Timeout"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::HttpStatus(c) => write!(f, "HttpStatus({c})"),
            Self::Network(m) => write!(f, "Network({m})"),
            Self::NotModified => write!(f, "NotModified"),
        }
    }
}

impl std::error::Error for FetchError {}

#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub status: u16,
    pub body: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub url: String,
    pub headers: HttpRequestHeaders,
    pub timeout: Duration,
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<String>,
    pub cancelled: bool,
}

impl FetchRequest {
    pub fn from_profile(profile: &SubscriptionProfile, timeout: Duration) -> Self {
        Self {
            url: profile.url.clone(),
            headers: profile.headers.clone(),
            timeout,
            if_none_match: profile.etag.clone(),
            if_modified_since: profile.last_modified.clone(),
            cancelled: false,
        }
    }
}

/// Trait so production can plug a real HTTP client later.
pub trait SubscriptionFetcher {
    fn fetch(&mut self, req: &FetchRequest) -> Result<FetchResponse, FetchError>;
}

/// In-memory fixture fetcher for tests and offline CI.
#[derive(Debug, Default)]
pub struct MockFetcher {
    /// url -> (body, etag)
    pub bodies: HashMap<String, (String, Option<String>)>,
    pub force_timeout: bool,
    pub force_status: Option<u16>,
}

impl MockFetcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&mut self, url: &str, body: &str, etag: Option<&str>) {
        self.bodies
            .insert(url.into(), (body.into(), etag.map(str::to_string)));
    }
}

impl SubscriptionFetcher for MockFetcher {
    fn fetch(&mut self, req: &FetchRequest) -> Result<FetchResponse, FetchError> {
        if req.cancelled {
            return Err(FetchError::Cancelled);
        }
        if self.force_timeout || req.timeout.is_zero() {
            return Err(FetchError::Timeout);
        }
        if let Some(code) = self.force_status {
            return Err(FetchError::HttpStatus(code));
        }
        let Some((body, etag)) = self.bodies.get(&req.url).cloned() else {
            return Err(FetchError::Network("no fixture for url".into()));
        };
        if let (Some(inm), Some(tag)) = (&req.if_none_match, &etag) {
            if inm == tag {
                return Err(FetchError::NotModified);
            }
        }
        let mut headers = HashMap::new();
        if let Some(t) = &etag {
            headers.insert("etag".into(), t.clone());
        }
        Ok(FetchResponse {
            status: 200,
            body,
            etag,
            last_modified: None,
            headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_modified() {
        let mut f = MockFetcher::new();
        f.seed("https://x/sub", "body", Some("etag1"));
        let mut req = FetchRequest {
            url: "https://x/sub".into(),
            headers: HttpRequestHeaders::default(),
            timeout: Duration::from_secs(5),
            if_none_match: Some("etag1".into()),
            if_modified_since: None,
            cancelled: false,
        };
        assert!(matches!(f.fetch(&req), Err(FetchError::NotModified)));
        req.if_none_match = None;
        assert_eq!(f.fetch(&req).unwrap().body, "body");
    }
}
