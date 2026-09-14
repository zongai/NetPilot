//! Proxy lifecycle manager (NP-033).

use std::collections::HashMap;

use crate::ProxyLifecycle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    FailedPrecondition {
        from: ProxyLifecycle,
        op: &'static str,
    },
    UnknownProxy(String),
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedPrecondition { from, op } => {
                write!(f, "FailedPrecondition: cannot {op} from {}", from.as_str())
            }
            Self::UnknownProxy(id) => write!(f, "UnknownProxy: {id}"),
        }
    }
}

impl std::error::Error for LifecycleError {}

/// Tracks lifecycle state per proxy id.
#[derive(Debug, Default)]
pub struct LifecycleManager {
    states: HashMap<String, ProxyLifecycle>,
}

impl LifecycleManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn ensure(&mut self, id: &str) -> ProxyLifecycle {
        *self
            .states
            .entry(id.to_string())
            .or_insert(ProxyLifecycle::Created)
    }

    pub fn state(&self, id: &str) -> Option<ProxyLifecycle> {
        self.states.get(id).copied()
    }

    pub fn begin_validate(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(
            id,
            &[ProxyLifecycle::Created, ProxyLifecycle::Stopped],
            ProxyLifecycle::Validating,
            "begin_validate",
        )
    }

    pub fn begin_start(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(
            id,
            &[
                ProxyLifecycle::Validating,
                ProxyLifecycle::Stopped,
                ProxyLifecycle::Created,
            ],
            ProxyLifecycle::Starting,
            "begin_start",
        )
    }

    pub fn mark_running(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(
            id,
            &[ProxyLifecycle::Starting],
            ProxyLifecycle::Running,
            "mark_running",
        )
    }

    pub fn begin_stop(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(
            id,
            &[
                ProxyLifecycle::Running,
                ProxyLifecycle::Starting,
                ProxyLifecycle::Validating,
            ],
            ProxyLifecycle::Stopping,
            "begin_stop",
        )
    }

    pub fn mark_stopped(&mut self, id: &str) -> Result<(), LifecycleError> {
        self.transition(
            id,
            &[ProxyLifecycle::Stopping],
            ProxyLifecycle::Stopped,
            "mark_stopped",
        )
    }

    pub fn mark_failed(&mut self, id: &str) -> Result<(), LifecycleError> {
        let cur = self.ensure(id);
        if matches!(cur, ProxyLifecycle::Stopped | ProxyLifecycle::Failed) {
            return Err(LifecycleError::FailedPrecondition {
                from: cur,
                op: "mark_failed",
            });
        }
        self.states.insert(id.to_string(), ProxyLifecycle::Failed);
        Ok(())
    }

    fn transition(
        &mut self,
        id: &str,
        allowed_from: &[ProxyLifecycle],
        next: ProxyLifecycle,
        op: &'static str,
    ) -> Result<(), LifecycleError> {
        let cur = self.ensure(id);
        if !allowed_from.contains(&cur) {
            return Err(LifecycleError::FailedPrecondition { from: cur, op });
        }
        self.states.insert(id.to_string(), next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let mut m = LifecycleManager::new();
        m.begin_validate("p1").unwrap();
        m.begin_start("p1").unwrap();
        m.mark_running("p1").unwrap();
        assert_eq!(m.state("p1"), Some(ProxyLifecycle::Running));
        m.begin_stop("p1").unwrap();
        m.mark_stopped("p1").unwrap();
        assert_eq!(m.state("p1"), Some(ProxyLifecycle::Stopped));
    }

    #[test]
    fn reject_running_from_created() {
        let mut m = LifecycleManager::new();
        assert!(matches!(
            m.mark_running("p1"),
            Err(LifecycleError::FailedPrecondition { .. })
        ));
    }
}
