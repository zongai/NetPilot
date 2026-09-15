//! Auto ProxyGroup from subscription (NP-138).

use netpilot_proxy::{GroupSelect, ProxyGroup, ProxyProfile};

pub fn group_from_subscription(
    subscription_id: &str,
    subscription_name: &str,
    profiles: &[ProxyProfile],
) -> ProxyGroup {
    let members: Vec<String> = profiles.iter().map(|p| p.id.clone()).collect();
    let mut g = ProxyGroup {
        id: format!("sub-group-{subscription_id}"),
        name: subscription_name.to_string(),
        select: GroupSelect::UrlTest,
        members,
        selected: None,
    };
    g.normalize_fields();
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_proxy::ProtocolKind;

    #[test]
    fn builds_group() {
        let p = ProxyProfile {
            id: "a".into(),
            name: "a".into(),
            protocol: ProtocolKind::Trojan,
            transport: None,
            server: "s".into(),
            port: 1,
            password: Some("x".into()),
            uuid: None,
            username: None,
            sni: None,
            alpn: None,
            path: None,
            host: None,
            flow: None,
            network: None,
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        };
        let g = group_from_subscription("s1", "Airport", &[p]);
        assert_eq!(g.members, vec!["a".to_string()]);
        assert_eq!(g.name, "Airport");
    }
}
