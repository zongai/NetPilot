//! Endpoint and credential validation (NP-034).

use crate::{ProtocolKind, ProxyProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointError {
    InvalidHost(String),
    InvalidPort(u16),
    InvalidUuid(String),
    MissingCredential(&'static str),
}

impl std::fmt::Display for EndpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHost(h) => write!(f, "InvalidHost: {h}"),
            Self::InvalidPort(p) => write!(f, "InvalidPort: {p}"),
            Self::InvalidUuid(u) => write!(f, "InvalidUuid: {u}"),
            Self::MissingCredential(c) => write!(f, "MissingCredential: {c}"),
        }
    }
}

impl std::error::Error for EndpointError {}

/// Validate host string: non-empty, no whitespace, reasonable length.
pub fn validate_host(host: &str) -> Result<(), EndpointError> {
    let h = host.trim();
    if h.is_empty() || h.len() > 253 {
        return Err(EndpointError::InvalidHost(host.to_string()));
    }
    if h.chars().any(|c| c.is_whitespace() || c == '/') {
        return Err(EndpointError::InvalidHost(host.to_string()));
    }
    Ok(())
}

pub fn validate_port(port: u16) -> Result<(), EndpointError> {
    if port == 0 {
        Err(EndpointError::InvalidPort(port))
    } else {
        Ok(())
    }
}

/// Accept standard 8-4-4-4-12 hex UUID (case-insensitive).
pub fn validate_uuid(uuid: &str) -> Result<(), EndpointError> {
    let u = uuid.trim();
    let parts: Vec<&str> = u.split('-').collect();
    if parts.len() != 5
        || parts[0].len() != 8
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 4
        || parts[4].len() != 12
        || !u.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err(EndpointError::InvalidUuid(uuid.to_string()));
    }
    Ok(())
}

/// Validate endpoint + protocol-appropriate credentials on a profile.
pub fn validate_profile_endpoint(profile: &ProxyProfile) -> Result<(), EndpointError> {
    validate_host(&profile.server)?;
    validate_port(profile.port)?;
    if profile.requires_uuid() {
        match &profile.uuid {
            Some(u) => validate_uuid(u)?,
            None => return Err(EndpointError::MissingCredential("uuid")),
        }
    }
    if matches!(profile.protocol, ProtocolKind::Trojan) && profile.password.is_none() {
        return Err(EndpointError::MissingCredential("password"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TransportKind;

    fn profile() -> ProxyProfile {
        ProxyProfile {
            id: "p".into(),
            name: "p".into(),
            protocol: ProtocolKind::Socks5,
            transport: None,
            server: "example.com".into(),
            port: 443,
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
    fn good_host_port() {
        validate_host("1.2.3.4").unwrap();
        validate_port(443).unwrap();
    }

    #[test]
    fn bad_host() {
        assert!(validate_host("").is_err());
        assert!(validate_host("a b").is_err());
    }

    #[test]
    fn uuid_format() {
        validate_uuid("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert!(validate_uuid("not-a-uuid").is_err());
    }

    #[test]
    fn vless_requires_uuid() {
        let mut p = profile();
        p.protocol = ProtocolKind::Vless;
        p.transport = Some(TransportKind::Reality);
        assert!(matches!(
            validate_profile_endpoint(&p),
            Err(EndpointError::MissingCredential("uuid"))
        ));
        p.uuid = Some("550e8400-e29b-41d4-a716-446655440000".into());
        validate_profile_endpoint(&p).unwrap();
    }
}
