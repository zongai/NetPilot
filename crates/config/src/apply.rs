//! Atomic runtime config apply (NP-036).

use crate::{ConfigError, NormalizedConfig};

/// Generation counter for applied configs (monotonic).
pub type ConfigGeneration = u64;

/// Holds the active runtime config with atomic replace semantics.
#[derive(Debug)]
pub struct RuntimeConfig {
    current: Option<NormalizedConfig>,
    generation: ConfigGeneration,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeConfig {
    pub fn new() -> Self {
        Self {
            current: None,
            generation: 0,
        }
    }

    pub fn generation(&self) -> ConfigGeneration {
        self.generation
    }

    pub fn current(&self) -> Option<&NormalizedConfig> {
        self.current.as_ref()
    }

    /// Atomically replace the active config. On success, generation increments.
    /// Validation is the caller's responsibility (`load_pipeline`); this only swaps.
    pub fn apply(&mut self, next: NormalizedConfig) -> Result<ConfigGeneration, ConfigError> {
        if next.schema_version == 0 {
            return Err(ConfigError::InvalidInput(
                "cannot apply schema_version 0".into(),
            ));
        }
        self.current = Some(next);
        self.generation = self.generation.saturating_add(1);
        Ok(self.generation)
    }

    /// Roll back to a previous snapshot (e.g. after a failed downstream activation).
    pub fn rollback(&mut self, previous: Option<NormalizedConfig>) -> ConfigGeneration {
        self.current = previous;
        self.generation = self.generation.saturating_add(1);
        self.generation
    }

    /// Snapshot for transactional apply: clone current, apply new, return guard data.
    pub fn begin_transaction(&self) -> ConfigTransaction {
        ConfigTransaction {
            previous: self.current.clone(),
            previous_generation: self.generation,
        }
    }
}

/// Snapshot taken before an apply; used to restore on failure.
#[derive(Debug, Clone)]
pub struct ConfigTransaction {
    pub previous: Option<NormalizedConfig>,
    pub previous_generation: ConfigGeneration,
}

impl ConfigTransaction {
    pub fn abort(self, runtime: &mut RuntimeConfig) -> ConfigGeneration {
        runtime.rollback(self.previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigDocument;

    #[test]
    fn apply_increments_generation() {
        let mut rt = RuntimeConfig::new();
        assert_eq!(rt.generation(), 0);
        let n = ConfigDocument::empty().load_pipeline().unwrap();
        let g1 = rt.apply(n.clone()).unwrap();
        assert_eq!(g1, 1);
        let g2 = rt.apply(n).unwrap();
        assert_eq!(g2, 2);
        assert!(rt.current().is_some());
    }

    #[test]
    fn transaction_rollback() {
        let mut rt = RuntimeConfig::new();
        let first = ConfigDocument::empty().load_pipeline().unwrap();
        rt.apply(first).unwrap();
        let tx = rt.begin_transaction();
        let mut second = ConfigDocument::empty();
        second.general.system_proxy = true;
        let second = second.load_pipeline().unwrap();
        rt.apply(second).unwrap();
        assert!(rt.current().unwrap().general.system_proxy);
        tx.abort(&mut rt);
        assert!(!rt.current().unwrap().general.system_proxy);
    }
}
