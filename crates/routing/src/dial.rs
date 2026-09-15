//! TCP dial policy (NP-176).

use std::time::Duration;

/// How Core should dial an outbound target after a routing decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialPolicy {
    pub connect_timeout: Duration,
    pub prefer_ipv4: bool,
    pub max_retries: u32,
}

impl Default for DialPolicy {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            prefer_ipv4: true,
            max_retries: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialTarget {
    pub host: String,
    pub port: u16,
    pub outbound: String,
}

impl DialTarget {
    pub fn new(host: impl Into<String>, port: u16, outbound: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            outbound: outbound.into(),
        }
    }
}

/// Validate dial target (no DNS here — host may be IP or name).
pub fn validate_dial_target(t: &DialTarget) -> Result<(), String> {
    if t.host.trim().is_empty() {
        return Err("empty host".into());
    }
    if t.port == 0 {
        return Err("invalid port".into());
    }
    if t.outbound.trim().is_empty() {
        return Err("empty outbound".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert!(validate_dial_target(&DialTarget::new("", 80, "DIRECT")).is_err());
        assert!(validate_dial_target(&DialTarget::new("h", 0, "DIRECT")).is_err());
    }

    #[test]
    fn ok_sample() {
        assert!(validate_dial_target(&DialTarget::new("example.com", 443, "PROXY")).is_ok());
    }
}
