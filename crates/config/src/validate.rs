//! Semantic validation (NP-028).

use crate::{ConfigDocument, ConfigError, ProtocolKind};

/// Semantic checks beyond structural ids/members.
pub fn validate_semantic(doc: &ConfigDocument) -> Result<(), ConfigError> {
    if let Some(port) = doc.general.mixed_port {
        if port == 0 {
            return Err(ConfigError::Semantic(
                "general.mixed_port must be non-zero when set".into(),
            ));
        }
    }

    for p in &doc.proxies {
        if p.port == 0 {
            return Err(ConfigError::Semantic(format!(
                "proxy {}: port must be 1..=65535",
                p.id
            )));
        }
        if p.server.is_empty() {
            return Err(ConfigError::Semantic(format!(
                "proxy {}: server empty",
                p.id
            )));
        }
        if p.requires_uuid() && p.uuid.as_ref().map(|u| u.trim().is_empty()).unwrap_or(true) {
            return Err(ConfigError::Semantic(format!(
                "proxy {}: protocol {:?} requires uuid",
                p.id, p.protocol
            )));
        }
        // REALITY is transport-only; must not be alone without a protocol that supports TLS-like path.
        if matches!(p.transport, Some(crate::TransportKind::Reality))
            && !matches!(
                p.protocol,
                ProtocolKind::Vless | ProtocolKind::Vmess | ProtocolKind::Trojan
            )
        {
            return Err(ConfigError::Semantic(format!(
                "proxy {}: reality transport only valid with vless/vmess/trojan",
                p.id
            )));
        }
    }

    for g in &doc.groups {
        if let Some(sel) = &g.selected {
            if !g.members.iter().any(|m| m == sel) {
                return Err(ConfigError::Semantic(format!(
                    "group {}: selected '{sel}' is not a member",
                    g.id
                )));
            }
        }
        if g.members.is_empty() {
            return Err(ConfigError::Semantic(format!(
                "group {}: members empty",
                g.id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GroupSelect, ProtocolKind, ProxyGroup, ProxyProfile, TransportKind};

    fn base_proxy(id: &str) -> ProxyProfile {
        ProxyProfile {
            id: id.into(),
            name: id.into(),
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
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        }
    }

    #[test]
    fn rejects_vless_without_uuid() {
        let mut doc = ConfigDocument::empty();
        let mut p = base_proxy("v1");
        p.protocol = ProtocolKind::Vless;
        doc.proxies.push(p);
        assert!(matches!(
            validate_semantic(&doc),
            Err(ConfigError::Semantic(_))
        ));
    }

    #[test]
    fn rejects_reality_on_socks() {
        let mut doc = ConfigDocument::empty();
        let mut p = base_proxy("s1");
        p.transport = Some(TransportKind::Reality);
        doc.proxies.push(p);
        assert!(matches!(
            validate_semantic(&doc),
            Err(ConfigError::Semantic(_))
        ));
    }

    #[test]
    fn rejects_selected_not_member() {
        let mut doc = ConfigDocument::empty();
        doc.proxies.push(base_proxy("p1"));
        doc.groups.push(ProxyGroup {
            id: "g1".into(),
            name: "g".into(),
            select: GroupSelect::Manual,
            members: vec!["p1".into()],
            selected: Some("other".into()),
        });
        assert!(matches!(
            validate_semantic(&doc),
            Err(ConfigError::Semantic(_))
        ));
    }

    #[test]
    fn ok_vless_with_uuid() {
        let mut doc = ConfigDocument::empty();
        let mut p = base_proxy("v1");
        p.protocol = ProtocolKind::Vless;
        p.uuid = Some("00000000-0000-0000-0000-000000000001".into());
        doc.proxies.push(p);
        validate_semantic(&doc).unwrap();
    }
}
