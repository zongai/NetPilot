//! Configuration: schema, load, normalize, validate, migrate (NP-025…NP-030).

#![forbid(unsafe_code)]

mod apply;
mod load;
mod migrate;
mod normalize;
mod validate;

pub use apply::{ConfigGeneration, ConfigTransaction, RuntimeConfig};
pub use load::{detect_format, load_from_path, load_from_str, ConfigFormat};
pub use migrate::{migrate_document, CURRENT_SCHEMA_VERSION};
pub use normalize::normalize_document;
pub use validate::validate_semantic;

pub use netpilot_proxy::{
    GroupSelect, ProtocolKind, ProxyGroup, ProxyLifecycle, ProxyProfile, TransportKind,
};

use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "netpilot-config";

/// Schema version constant (alias of migration current).
pub const CONFIG_SCHEMA_VERSION: u32 = CURRENT_SCHEMA_VERSION;

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

/// Validated / normalized view after load + normalize + semantic checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedConfig {
    pub schema_version: u32,
    pub proxies: Vec<ProxyProfile>,
    pub groups: Vec<ProxyGroup>,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    InvalidInput(String),
    Parse(String),
    Io(String),
    DuplicateId(String),
    MissingMember { group: String, member: String },
    Semantic(String),
    UnsupportedVersion(u32),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(m) => write!(f, "InvalidInput: {m}"),
            Self::Parse(m) => write!(f, "Parse: {m}"),
            Self::Io(m) => write!(f, "Io: {m}"),
            Self::DuplicateId(id) => write!(f, "DuplicateId: {id}"),
            Self::MissingMember { group, member } => {
                write!(f, "MissingMember: group={group} member={member}")
            }
            Self::Semantic(m) => write!(f, "Semantic: {m}"),
            Self::UnsupportedVersion(v) => write!(f, "UnsupportedVersion: {v}"),
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

    /// Structural validation (ids, members).
    pub fn validate_structure(&self) -> Result<(), ConfigError> {
        if self.schema_version == 0 {
            return Err(ConfigError::InvalidInput(
                "schema_version must be >= 1".into(),
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for p in &self.proxies {
            if p.id.is_empty() || p.server.is_empty() || p.port == 0 {
                return Err(ConfigError::InvalidInput(
                    "proxy requires id, server, non-zero port".into(),
                ));
            }
            if !ids.insert(p.id.clone()) {
                return Err(ConfigError::DuplicateId(p.id.clone()));
            }
        }
        for g in &self.groups {
            if g.id.is_empty() {
                return Err(ConfigError::InvalidInput("group id empty".into()));
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

    /// Full pipeline: migrate → normalize → structure → semantic → NormalizedConfig.
    pub fn load_pipeline(self) -> Result<NormalizedConfig, ConfigError> {
        let migrated = migrate_document(self)?;
        let normalized = normalize_document(migrated);
        normalized.validate_structure()?;
        validate_semantic(&normalized)?;
        Ok(NormalizedConfig {
            schema_version: normalized.schema_version,
            proxies: normalized.proxies,
            groups: normalized.groups,
            general: normalized.general,
        })
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        serde_json::to_string_pretty(self).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    pub fn to_yaml(&self) -> Result<String, ConfigError> {
        serde_yaml::to_string(self).map_err(|e| ConfigError::Parse(e.to_string()))
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
    fn empty_pipeline() {
        let n = ConfigDocument::empty().load_pipeline().unwrap();
        assert_eq!(n.schema_version, CONFIG_SCHEMA_VERSION);
    }

    #[test]
    fn json_load_and_pipeline() {
        let mut doc = ConfigDocument::empty();
        doc.proxies.push(sample_proxy("p1"));
        let json = doc.to_json().unwrap();
        let loaded = load_from_str(&json, ConfigFormat::Json).unwrap();
        let n = loaded.load_pipeline().unwrap();
        assert_eq!(n.proxies.len(), 1);
    }

    #[test]
    fn yaml_load() {
        let yaml = r#"
schema_version: 1
proxies:
  - id: p1
    name: local
    protocol: socks5
    server: 127.0.0.1
    port: 1080
groups: []
general: {}
"#;
        let doc = load_from_str(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(doc.proxies[0].id, "p1");
        doc.load_pipeline().unwrap();
    }

    #[test]
    fn duplicate_id() {
        let mut doc = ConfigDocument::empty();
        doc.proxies.push(sample_proxy("p1"));
        doc.proxies.push(sample_proxy("p1"));
        assert!(matches!(
            doc.validate_structure(),
            Err(ConfigError::DuplicateId(_))
        ));
    }
}
