//! Atomic RuleSet update (NP-095).

use crate::cache::RuleSetCache;
use crate::loader::{load_ruleset_from_str, LoadError};
use crate::ruleset::RuleSet;

#[derive(Debug)]
pub struct RuleSetUpdater {
    active: Option<RuleSet>,
    generation: u64,
    cache: RuleSetCache,
}

impl Default for RuleSetUpdater {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleSetUpdater {
    pub fn new() -> Self {
        Self {
            active: None,
            generation: 0,
            cache: RuleSetCache::new(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn active(&self) -> Option<&RuleSet> {
        self.active.as_ref()
    }

    /// Replace active set atomically after successful parse.
    pub fn apply_text(&mut self, id: &str, name: &str, text: &str) -> Result<u64, LoadError> {
        let set = load_ruleset_from_str(id, name, text)?;
        self.cache
            .put(set.clone(), std::time::Duration::from_secs(3600));
        self.active = Some(set);
        self.generation = self.generation.saturating_add(1);
        Ok(self.generation)
    }

    pub fn rollback(&mut self, previous: Option<RuleSet>) -> u64 {
        self.active = previous;
        self.generation = self.generation.saturating_add(1);
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_apply() {
        let mut u = RuleSetUpdater::new();
        let g = u.apply_text("d", "D", "MATCH,DIRECT\n").unwrap();
        assert_eq!(g, 1);
        assert_eq!(u.active().unwrap().len(), 1);
    }
}
