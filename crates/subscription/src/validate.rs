//! Security checks for subscription configs (NP-141).

use netpilot_proxy::ProxyProfile;

use crate::profile::SubscriptionProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityIssue {
    InsecureUrl,
    EmptyServer,
    ZeroPort,
    MissingCredential,
}

pub fn validate_subscription_security(
    profile: &SubscriptionProfile,
    nodes: &[ProxyProfile],
) -> Vec<SecurityIssue> {
    let mut issues = Vec::new();
    if profile.url.starts_with("http://") {
        issues.push(SecurityIssue::InsecureUrl);
    }
    for n in nodes {
        if n.server.trim().is_empty() {
            issues.push(SecurityIssue::EmptyServer);
        }
        if n.port == 0 {
            issues.push(SecurityIssue::ZeroPort);
        }
        if n.expects_password()
            && n.password.as_ref().map(|s| s.is_empty()).unwrap_or(true)
            && !matches!(n.protocol, netpilot_proxy::ProtocolKind::Http)
        {
            issues.push(SecurityIssue::MissingCredential);
        }
        if n.requires_uuid() && n.uuid.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            issues.push(SecurityIssue::MissingCredential);
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_http_url() {
        let p = SubscriptionProfile::new("1", "n", "http://insecure.example/sub");
        let issues = validate_subscription_security(&p, &[]);
        assert!(issues.contains(&SecurityIssue::InsecureUrl));
    }
}
