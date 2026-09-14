//! Multi-subscription merge (NP-139).

use netpilot_proxy::ProxyProfile;

use crate::node_fingerprint::fingerprint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflict {
    pub fingerprint: String,
    pub kept_id: String,
    pub dropped_id: String,
}

pub fn merge_profiles(sets: Vec<Vec<ProxyProfile>>) -> (Vec<ProxyProfile>, Vec<MergeConflict>) {
    let mut out = Vec::new();
    let mut conflicts = Vec::new();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for set in sets {
        for p in set {
            let fp = fingerprint(&p);
            if let Some(prev) = seen.get(&fp) {
                conflicts.push(MergeConflict {
                    fingerprint: fp,
                    kept_id: prev.clone(),
                    dropped_id: p.id,
                });
            } else {
                seen.insert(fp, p.id.clone());
                out.push(p);
            }
        }
    }
    (out, conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_proxy::ProtocolKind;

    fn p(id: &str, server: &str) -> ProxyProfile {
        ProxyProfile {
            id: id.into(),
            name: id.into(),
            protocol: ProtocolKind::Socks5,
            transport: None,
            server: server.into(),
            port: 1080,
            password: None,
            uuid: None,
            username: None,
            sni: None,
            alpn: None,
            path: None,
            host: None,
            flow: None,
            network: None,
            tags: vec![],
        }
    }

    #[test]
    fn conflict_recorded() {
        let (merged, conflicts) = merge_profiles(vec![vec![p("1", "h")], vec![p("2", "h")]]);
        assert_eq!(merged.len(), 1);
        assert_eq!(conflicts.len(), 1);
    }
}
