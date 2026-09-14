//! Canonical configuration schema (NP-025).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub use netpilot_proxy::{
    GroupSelect, ProtocolKind, ProxyGroup, ProxyLifecycle, ProxyProfile, TransportKind,
};

pub const CRATE_NAME: &str = "netpilot-config";

/// Schema version for migrations (later NP-029).
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Top-level document as authored by user / Desktop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub proxies: Vec<ProxyProfile>,
    #[serde(default)]
    pub groups: Vec<ProxyGroup>,
    #[serde(default)]
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(default)]
    pub system_proxy: bool,
    #[serde(default)]
    pub allow_lan: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixed_port: Option<u16>,
}

/// Validated / normalized view after load (NP-026+ will fill loaders).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedConfig {
    pub schema_version: u32,
    pub proxies: Vec<ProxyProfile>,
    pub groups: Vec<ProxyGroup>,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    InvalidInput(&'static str),
    DuplicateId(String),
    MissingMember { group: String, member: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(m) => write!(f, "InvalidInput: {m}"),
            Self::DuplicateId(id) => write!(f, "DuplicateId: {id}"),
            Self::MissingMember { group, member } => {
                write!(f, "MissingMember: group={group} member={member}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl ConfigDocument {
    pub fn empty() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            proxies: vec![],
            groups: vec![],
            general: GeneralConfig::default(),
        }
    }

    /// Structural validation (no I/O).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version == 0 {
            return Err(ConfigError::InvalidInput("schema_version must be >= 1"));
        }
        let mut ids = std::collections::HashSet::new();
        for p in &self.proxies {
            if p.id.is_empty() || p.server.is_empty() || p.port == 0 {
                return Err(ConfigError::InvalidInput(
                    "proxy requires id, server, non-zero port",
                ));
            }
            if !ids.insert(p.id.clone()) {
                return Err(ConfigError::DuplicateId(p.id.clone()));
            }
        }
        for g in &self.groups {
            if g.id.is_empty() {
                return Err(ConfigError::InvalidInput("group id empty"));
            }
            if !ids.insert(g.id.clone()) {
                return Err(ConfigError::DuplicateId(g.id.clone()));
            }
            for m in &g.members {
                if !self.proxies.iter().any(|p| p.id == *m)
                    && !self.groups.iter().any(|og| og.id == *m && og.id != g.id)
                {
                    return Err(ConfigError::MissingMember {
                        group: g.id.clone(),
                        member: m.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn normalize(self) -> Result<NormalizedConfig, ConfigError> {
        self.validate()?;
        Ok(NormalizedConfig {
            schema_version: self.schema_version,
            proxies: self.proxies,
            groups: self.groups,
            general: self.general,
        })
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_proxy(id: &str) -> ProxyProfile {
        ProxyProfile {
            id: id.into(),
            name: id.into(),
            protocol: ProtocolKind::Socks5,
            transport: None,
            server: "127.0.0.1".into(),
            port: 1080,
            password: None,
            uuid: None,
            tags: vec![],
        }
    }

    #[test]
    fn empty_valid() {
        ConfigDocument::empty().validate().unwrap();
    }

    #[test]
    fn normalize_ok() {
        let mut doc = ConfigDocument::empty();
        doc.proxies.push(sample_proxy("p1"));
        doc.groups.push(ProxyGroup {
            id: "g1".into(),
            name: "default".into(),
            select: GroupSelect::Manual,
            members: vec!["p1".into()],
            selected: Some("p1".into()),
        });
        let n = doc.normalize().unwrap();
        assert_eq!(n.proxies.len(), 1);
    }

    #[test]
    fn duplicate_id() {
        let mut doc = ConfigDocument::empty();
        doc.proxies.push(sample_proxy("p1"));
        doc.proxies.push(sample_proxy("p1"));
        assert!(matches!(doc.validate(), Err(ConfigError::DuplicateId(_))));
    }

    #[test]
    fn missing_member() {
        let mut doc = ConfigDocument::empty();
        doc.groups.push(ProxyGroup {
            id: "g1".into(),
            name: "g".into(),
            select: GroupSelect::Manual,
            members: vec!["nope".into()],
            selected: None,
        });
        assert!(matches!(
            doc.validate(),
            Err(ConfigError::MissingMember { .. })
        ));
    }

    #[test]
    fn json_roundtrip() {
        let doc = ConfigDocument::empty();
        let s = doc.to_json().unwrap();
        let back = ConfigDocument::from_json(&s).unwrap();
        assert_eq!(back.schema_version, CONFIG_SCHEMA_VERSION);
    }
}
