//! YAML/JSON loading (NP-026).

use std::path::Path;

use crate::{ConfigDocument, ConfigError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Yaml,
}

/// Detect format from path extension (`.json` / `.yaml` / `.yml`).
pub fn detect_format(path: &Path) -> Result<ConfigFormat, ConfigError> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => Ok(ConfigFormat::Json),
        Some("yaml") | Some("yml") => Ok(ConfigFormat::Yaml),
        _ => Err(ConfigError::InvalidInput(
            "unsupported config extension (use .json/.yaml/.yml)".into(),
        )),
    }
}

/// Parse a config document from text.
pub fn load_from_str(text: &str, format: ConfigFormat) -> Result<ConfigDocument, ConfigError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ConfigError::InvalidInput("empty config text".into()));
    }
    match format {
        ConfigFormat::Json => {
            serde_json::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
        }
        ConfigFormat::Yaml => {
            serde_yaml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))
        }
    }
}

/// Load from filesystem path; format from extension.
pub fn load_from_path(path: &Path) -> Result<ConfigDocument, ConfigError> {
    let format = detect_format(path)?;
    let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
    // Avoid logging path content; path itself is fine in errors.
    load_from_str(&text, format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_ext() {
        assert_eq!(
            detect_format(Path::new("a.json")).unwrap(),
            ConfigFormat::Json
        );
        assert_eq!(
            detect_format(Path::new("a.YAML")).unwrap(),
            ConfigFormat::Yaml
        );
        assert!(detect_format(Path::new("a.txt")).is_err());
    }

    #[test]
    fn parse_json_minimal() {
        let doc = load_from_str(r#"{"schema_version":1}"#, ConfigFormat::Json).unwrap();
        assert_eq!(doc.schema_version, 1);
        assert!(doc.proxies.is_empty());
    }

    #[test]
    fn parse_yaml_minimal() {
        let doc = load_from_str("schema_version: 1\n", ConfigFormat::Yaml).unwrap();
        assert_eq!(doc.schema_version, 1);
    }

    #[test]
    fn reject_empty() {
        assert!(matches!(
            load_from_str("  ", ConfigFormat::Json),
            Err(ConfigError::InvalidInput(_))
        ));
    }

    #[test]
    fn path_missing_is_io() {
        let p = PathBuf::from("/nonexistent/netpilot-config-xyz.json");
        assert!(matches!(load_from_path(&p), Err(ConfigError::Io(_))));
    }
}
