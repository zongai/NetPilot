//! Local / bypass traffic policy (NP-070).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassReason {
    Loopback,
    LinkLocal,
    PrivateLan,
    Broadcast,
    PolicyAllow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BypassDecision {
    /// Do not intercept; leave to OS stack.
    Bypass(BypassReason),
    /// Subject to TUN / proxy path.
    Intercept,
}

/// Decides whether a destination should skip the tunnel.
#[derive(Debug, Clone)]
pub struct BypassPolicy {
    pub bypass_loopback: bool,
    pub bypass_private: bool,
    pub bypass_link_local: bool,
}

impl Default for BypassPolicy {
    fn default() -> Self {
        Self {
            bypass_loopback: true,
            bypass_private: false,
            bypass_link_local: true,
        }
    }
}

impl BypassPolicy {
    pub fn decide(&self, dest_ip: &str, _port: u16) -> BypassDecision {
        let ip = dest_ip.trim();
        if self.bypass_loopback && (ip.starts_with("127.") || ip == "::1" || ip == "[::1]") {
            return BypassDecision::Bypass(BypassReason::Loopback);
        }
        if self.bypass_link_local
            && (ip.starts_with("169.254.") || ip.to_ascii_lowercase().starts_with("fe80:"))
        {
            return BypassDecision::Bypass(BypassReason::LinkLocal);
        }
        if self.bypass_private && is_private_v4(ip) {
            return BypassDecision::Bypass(BypassReason::PrivateLan);
        }
        if ip.ends_with(".255") || ip == "255.255.255.255" {
            return BypassDecision::Bypass(BypassReason::Broadcast);
        }
        BypassDecision::Intercept
    }
}

fn is_private_v4(ip: &str) -> bool {
    ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || (ip.starts_with("172.") && {
            // 172.16.0.0 – 172.31.255.255
            let mut parts = ip.split('.');
            let second = parts.nth(1).and_then(|s| s.parse::<u8>().ok());
            matches!(second, Some(n) if (16..=31).contains(&n))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_bypassed() {
        let p = BypassPolicy::default();
        assert!(matches!(
            p.decide("127.0.0.1", 80),
            BypassDecision::Bypass(BypassReason::Loopback)
        ));
    }

    #[test]
    fn public_intercepted() {
        let p = BypassPolicy::default();
        assert_eq!(p.decide("8.8.8.8", 53), BypassDecision::Intercept);
    }

    #[test]
    fn private_optional() {
        let mut p = BypassPolicy::default();
        assert_eq!(p.decide("192.168.1.1", 80), BypassDecision::Intercept);
        p.bypass_private = true;
        assert!(matches!(
            p.decide("192.168.1.1", 80),
            BypassDecision::Bypass(BypassReason::PrivateLan)
        ));
    }
}
