//! Name normalize & rename (NP-137).

use netpilot_proxy::ProxyProfile;

#[derive(Debug, Clone, Default)]
pub struct RenameRule {
    /// prefix added to display name
    pub prefix: Option<String>,
    pub replace: Vec<(String, String)>,
}

pub fn apply_rename(mut profiles: Vec<ProxyProfile>, rule: &RenameRule) -> Vec<ProxyProfile> {
    for p in &mut profiles {
        let mut name = p.name.trim().to_string();
        for (a, b) in &rule.replace {
            name = name.replace(a, b);
        }
        if let Some(ref pre) = rule.prefix {
            if !name.starts_with(pre) {
                name = format!("{pre}{name}");
            }
        }
        // collapse whitespace
        name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        p.name = name;
    }
    profiles
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_proxy::ProtocolKind;

    #[test]
    fn prefix() {
        let p = ProxyProfile {
            id: "1".into(),
            name: "  node  ".into(),
            protocol: ProtocolKind::Http,
            transport: None,
            server: "h".into(),
            port: 80,
            password: None,
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
        let rule = RenameRule {
            prefix: Some("[sub] ".into()),
            replace: vec![],
        };
        let out = apply_rename(vec![p], &rule);
        assert_eq!(out[0].name, "[sub] node");
    }
}
