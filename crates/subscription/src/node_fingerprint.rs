//! Stable node fingerprint and dedup (NP-135).

use netpilot_proxy::ProxyProfile;

pub fn fingerprint(p: &ProxyProfile) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let material = format!(
        "{}|{}|{}|{}|{}",
        p.protocol.as_str(),
        p.server.to_ascii_lowercase(),
        p.port,
        p.uuid.as_deref().unwrap_or(""),
        p.transport.as_ref().map(|t| t.as_str()).unwrap_or("tcp")
    );
    for b in material.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// Keep first occurrence of each fingerprint.
pub fn merge_duplicates(profiles: Vec<ProxyProfile>) -> Vec<ProxyProfile> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in profiles {
        let fp = fingerprint(&p);
        if seen.insert(fp) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_proxy::ProtocolKind;

    fn sample(server: &str, name: &str) -> ProxyProfile {
        ProxyProfile {
            id: name.into(),
            name: name.into(),
            protocol: ProtocolKind::Trojan,
            transport: None,
            server: server.into(),
            port: 443,
            password: Some("x".into()),
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
    fn dedup() {
        let v = vec![
            sample("a.com", "1"),
            sample("a.com", "2"),
            sample("b.com", "3"),
        ];
        assert_eq!(merge_duplicates(v).len(), 2);
    }
}
