//! Config normalization (NP-027).

use crate::ConfigDocument;

/// Trim fields, fill default names, dedupe tags, stable ordering of lists by id.
pub fn normalize_document(mut doc: ConfigDocument) -> ConfigDocument {
    for p in &mut doc.proxies {
        p.normalize_fields();
    }
    for g in &mut doc.groups {
        g.normalize_fields();
    }
    doc.proxies.sort_by(|a, b| a.id.cmp(&b.id));
    doc.groups.sort_by(|a, b| a.id.cmp(&b.id));
    if doc.schema_version == 0 {
        doc.schema_version = crate::CONFIG_SCHEMA_VERSION;
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProtocolKind, ProxyProfile};

    #[test]
    fn trims_and_sorts() {
        let mut doc = ConfigDocument::empty();
        doc.proxies.push(ProxyProfile {
            id: " b ".into(),
            name: "  ".into(),
            protocol: ProtocolKind::Http,
            transport: None,
            server: " x.com ".into(),
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
            tags: vec!["z".into(), " a ".into(), "a".into()],
        });
        doc.proxies.push(ProxyProfile {
            id: "a".into(),
            name: "A".into(),
            protocol: ProtocolKind::Socks5,
            transport: None,
            server: "127.0.0.1".into(),
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
        });
        let n = normalize_document(doc);
        assert_eq!(n.proxies[0].id, "a");
        assert_eq!(n.proxies[1].id, "b");
        assert_eq!(n.proxies[1].name, "b");
        assert_eq!(n.proxies[1].server, "x.com");
        assert_eq!(n.proxies[1].tags, vec!["a".to_string(), "z".to_string()]);
    }
}
