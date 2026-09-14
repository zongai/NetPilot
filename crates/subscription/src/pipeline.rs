//! End-to-end subscription processing (NP-144 support).

use netpilot_proxy::{ProxyGroup, ProxyProfile};

use crate::decoder::{detect_and_decode, DecodedBody};
use crate::filter::{apply_filters, FilterRule};
use crate::groups::group_from_subscription;
use crate::merge::merge_profiles;
use crate::node_fingerprint::merge_duplicates;
use crate::normalizer::normalize_nodes;
use crate::parsers::{parse_clash_yaml, parse_singbox_json, parse_uri_list, ParsedNode};
use crate::profile::SubscriptionProfile;
use crate::rename::{apply_rename, RenameRule};
use crate::validate::validate_subscription_security;

#[derive(Debug)]
pub struct PipelineResult {
    pub profiles: Vec<ProxyProfile>,
    pub group: ProxyGroup,
    pub security_issues: usize,
}

pub fn run_subscription_pipeline(
    sub: &SubscriptionProfile,
    body: &str,
    filter: &FilterRule,
    rename: &RenameRule,
) -> PipelineResult {
    let decoded = detect_and_decode(body).unwrap_or(DecodedBody::Plain(body.to_string()));
    let text = match &decoded {
        DecodedBody::Plain(s) | DecodedBody::Base64(s) => s.as_str(),
    };
    let mut nodes: Vec<ParsedNode> = Vec::new();
    if text.contains("proxies:") {
        nodes.extend(parse_clash_yaml(text));
    } else if text.trim_start().starts_with('{') {
        nodes.extend(parse_singbox_json(text));
    } else {
        for n in parse_uri_list(text).into_iter().flatten() {
            nodes.push(n);
        }
    }
    let mut profiles = normalize_nodes(&nodes, &sub.id);
    profiles = merge_duplicates(profiles);
    profiles = apply_filters(profiles, filter);
    profiles = apply_rename(profiles, rename);
    let issues = validate_subscription_security(sub, &profiles);
    let group = group_from_subscription(&sub.id, &sub.name, &profiles);
    let _ = merge_profiles(vec![profiles.clone()]);
    PipelineResult {
        profiles,
        group,
        security_issues: issues.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_pipeline() {
        let sub = SubscriptionProfile::new("s1", "Test", "https://example.com/sub");
        let body = "trojan://pass@host.example:443?security=tls#HK-1\n";
        let r =
            run_subscription_pipeline(&sub, body, &FilterRule::default(), &RenameRule::default());
        assert_eq!(r.profiles.len(), 1);
        assert_eq!(r.group.members.len(), 1);
    }
}
