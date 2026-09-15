//! Runtime data directories (NP-151).
//!
//! Resolves portable paths for config, cache, logs, and instance lock.
//! Never embeds secrets; paths are local filesystem only.

use std::path::PathBuf;

/// Well-known subdirectories under the NetPilot data root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub root: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub run_dir: PathBuf,
}

impl RuntimePaths {
    /// Resolve from environment or platform defaults.
    ///
    /// Order:
    /// 1. `NETPILOT_DATA_DIR`
    /// 2. Windows: `%LOCALAPPDATA%\NetPilot`
    /// 3. Unix: `$XDG_DATA_HOME/netpilot` or `~/.local/share/netpilot`
    /// 4. Fallback: `./.netpilot-data`
    pub fn resolve() -> Self {
        if let Ok(dir) = std::env::var("NETPILOT_DATA_DIR") {
            return Self::from_root(PathBuf::from(dir));
        }
        #[cfg(windows)]
        {
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                return Self::from_root(PathBuf::from(local).join("NetPilot"));
            }
        }
        #[cfg(unix)]
        {
            if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
                return Self::from_root(PathBuf::from(xdg).join("netpilot"));
            }
            if let Ok(home) = std::env::var("HOME") {
                return Self::from_root(
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join("netpilot"),
                );
            }
        }
        Self::from_root(PathBuf::from(".netpilot-data"))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self {
            config_dir: root.join("config"),
            cache_dir: root.join("cache"),
            log_dir: root.join("logs"),
            run_dir: root.join("run"),
            root,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    pub fn instance_lock(&self) -> PathBuf {
        self.run_dir.join("core.instance.lock")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        std::fs::create_dir_all(&self.run_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_root_layout() {
        let p = RuntimePaths::from_root(PathBuf::from("/tmp/np-test-root"));
        assert_eq!(
            p.config_file(),
            PathBuf::from("/tmp/np-test-root/config/config.json")
        );
        assert!(p.instance_lock().ends_with("core.instance.lock"));
    }

    #[test]
    fn resolve_respects_env() {
        std::env::set_var("NETPILOT_DATA_DIR", "/tmp/netpilot-env-root");
        let p = RuntimePaths::resolve();
        assert_eq!(p.root, PathBuf::from("/tmp/netpilot-env-root"));
        std::env::remove_var("NETPILOT_DATA_DIR");
    }
}
