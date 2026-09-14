//! Secret storage boundary and redaction (NP-035).

use crate::ProxyProfile;

/// Fields treated as secrets (must not appear in logs / IPC dumps).
pub const SECRET_FIELD_NAMES: &[&str] = &["password", "uuid", "username"];

/// Redacted placeholder used in exports.
pub const REDACTED: &str = "***";

/// Return a clone with secret fields replaced by [`REDACTED`].
pub fn redact_profile(profile: &ProxyProfile) -> ProxyProfile {
    let mut p = profile.clone();
    if p.password.is_some() {
        p.password = Some(REDACTED.into());
    }
    if p.uuid.is_some() {
        p.uuid = Some(REDACTED.into());
    }
    if p.username.is_some() {
        p.username = Some(REDACTED.into());
    }
    p
}

/// True if `text` appears to contain any secret value from the profile.
pub fn leaks_secret(profile: &ProxyProfile, text: &str) -> bool {
    for secret in [&profile.password, &profile.uuid, &profile.username]
        .into_iter()
        .flatten()
    {
        if !secret.is_empty() && text.contains(secret.as_str()) {
            return true;
        }
    }
    false
}

/// Strip secrets for safe diagnostic maps (key → value).
pub fn redacted_field_map(profile: &ProxyProfile) -> Vec<(&'static str, String)> {
    let mut out = vec![
        ("id", profile.id.clone()),
        ("name", profile.name.clone()),
        ("protocol", profile.protocol.as_str().to_string()),
        ("server", profile.server.clone()),
        ("port", profile.port.to_string()),
    ];
    if profile.password.is_some() {
        out.push(("password", REDACTED.into()));
    }
    if profile.uuid.is_some() {
        out.push(("uuid", REDACTED.into()));
    }
    if profile.username.is_some() {
        out.push(("username", REDACTED.into()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProtocolKind, TransportKind};

    fn secret_profile() -> ProxyProfile {
        ProxyProfile {
            id: "n1".into(),
            name: "node".into(),
            protocol: ProtocolKind::Vless,
            transport: Some(TransportKind::Tls),
            server: "example.com".into(),
            port: 443,
            password: Some("s3cr3t-pass".into()),
            uuid: Some("550e8400-e29b-41d4-a716-446655440000".into()),
            username: Some("alice".into()),
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
    fn redact_replaces_secrets() {
        let r = redact_profile(&secret_profile());
        assert_eq!(r.password.as_deref(), Some(REDACTED));
        assert_eq!(r.uuid.as_deref(), Some(REDACTED));
        assert_eq!(r.server, "example.com");
    }

    #[test]
    fn leak_detection() {
        let p = secret_profile();
        assert!(leaks_secret(&p, "user used s3cr3t-pass"));
        assert!(!leaks_secret(&p, p.redacted_summary().as_str()));
    }

    #[test]
    fn field_map_has_no_plaintext_secret() {
        let p = secret_profile();
        let map = redacted_field_map(&p);
        let blob = map
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(!blob.contains("s3cr3t-pass"));
        assert!(!blob.contains("550e8400"));
        assert!(blob.contains(REDACTED));
    }
}
