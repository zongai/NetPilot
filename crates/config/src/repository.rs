//! Config repository (NP-170).

use crate::{load_from_path, load_from_str, ConfigDocument, ConfigFormat, RuntimePaths};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigRepository {
    paths: RuntimePaths,
}

impl ConfigRepository {
    pub fn new(paths: RuntimePaths) -> Self {
        Self { paths }
    }

    pub fn from_env() -> Self {
        Self::new(RuntimePaths::resolve())
    }

    pub fn config_path(&self) -> PathBuf {
        self.paths.config_file()
    }

    pub fn load(&self) -> Result<ConfigDocument, String> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(ConfigDocument::empty());
        }
        load_from_path(&path).map_err(|e| e.to_string())
    }

    pub fn save(&self, doc: &ConfigDocument) -> Result<(), String> {
        self.paths
            .ensure_dirs()
            .map_err(|e| format!("ensure dirs: {e}"))?;
        let path = self.config_path();
        let text = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
        fs::write(&path, text).map_err(|e| format!("write config: {e}"))
    }

    pub fn load_str(text: &str, format: ConfigFormat) -> Result<ConfigDocument, String> {
        load_from_str(text, format).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn save_load_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("np-cfg-repo-{stamp}"));
        let paths = RuntimePaths::from_root(root.clone());
        let repo = ConfigRepository::new(paths);
        let doc = ConfigDocument::empty();
        repo.save(&doc).unwrap();
        let loaded = repo.load().unwrap();
        assert_eq!(loaded, doc);
        let _ = fs::remove_dir_all(root);
    }
}
