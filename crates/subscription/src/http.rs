//! HTTP headers & User-Agent (NP-124).

use std::collections::HashMap;

pub const DEFAULT_USER_AGENT: &str = "NetPilot/0.1 (subscription)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestHeaders {
    pub user_agent: String,
    /// Extra headers (Authorization etc.). Values may hold secrets — never log raw.
    pub extra: HashMap<String, String>,
}

impl Default for HttpRequestHeaders {
    fn default() -> Self {
        Self {
            user_agent: DEFAULT_USER_AGENT.into(),
            extra: HashMap::new(),
        }
    }
}

impl HttpRequestHeaders {
    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.extra.insert(name.into(), value.into());
    }

    pub fn authorization_bearer(&mut self, token: &str) {
        self.extra
            .insert("Authorization".into(), format!("Bearer {token}"));
    }

    /// Redacted view for logs.
    pub fn redacted_pairs(&self) -> Vec<(String, String)> {
        let mut out = vec![("User-Agent".into(), self.user_agent.clone())];
        for (k, v) in &self.extra {
            let lower = k.to_ascii_lowercase();
            let val = if lower.contains("auth")
                || lower.contains("token")
                || lower.contains("password")
            {
                "[redacted]".into()
            } else {
                v.clone()
            };
            out.push((k.clone(), val));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_auth() {
        let mut h = HttpRequestHeaders::default();
        h.authorization_bearer("secret-token");
        let pairs = h.redacted_pairs();
        assert!(pairs.iter().any(|(_, v)| v == "[redacted]"));
    }
}
