//! Node filters (NP-136).

use netpilot_proxy::ProxyProfile;

#[derive(Debug, Clone, Default)]
pub struct FilterRule {
    pub name_contains: Option<String>,
    pub protocol: Option<String>,
    pub region_keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
}

pub fn apply_filters(profiles: Vec<ProxyProfile>, rule: &FilterRule) -> Vec<ProxyProfile> {
    profiles
        .into_iter()
        .filter(|p| match_one(p, rule))
        .collect()
}

fn match_one(p: &ProxyProfile, rule: &FilterRule) -> bool {
    if let Some(ref n) = rule.name_contains {
        if !p
            .name
            .to_ascii_lowercase()
            .contains(&n.to_ascii_lowercase())
        {
            return false;
        }
    }
    if let Some(ref proto) = rule.protocol {
        if !p.protocol.as_str().eq_ignore_ascii_case(proto) {
            return false;
        }
    }
    let name_l = p.name.to_ascii_lowercase();
    for ex in &rule.exclude_keywords {
        if name_l.contains(&ex.to_ascii_lowercase()) {
            return false;
        }
    }
    if !rule.region_keywords.is_empty() {
        let ok = rule
            .region_keywords
            .iter()
            .any(|k| name_l.contains(&k.to_ascii_lowercase()));
        if !ok {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_proxy::ProtocolKind;

    #[test]
    fn exclude_and_region() {
        let p = ProxyProfile {
            id: "1".into(),
            name: "HK-01".into(),
            protocol: ProtocolKind::Vless,
            transport: None,
            server: "x".into(),
            port: 1,
            password: None,
            uuid: Some("u".into()),
            username: None,
            sni: None,
            alpn: None,
            path: None,
            host: None,
            flow: None,
            network: None,
            tags: vec![],
        };
        let rule = FilterRule {
            region_keywords: vec!["HK".into()],
            exclude_keywords: vec!["exp".into()],
            ..Default::default()
        };
        assert_eq!(apply_filters(vec![p], &rule).len(), 1);
    }
}
