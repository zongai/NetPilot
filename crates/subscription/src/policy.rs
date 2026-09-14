//! Update policy (NP-127).

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateTrigger {
    Manual,
    OnStart,
    Interval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePolicy {
    pub on_start: bool,
    pub interval: Option<Duration>,
}

impl Default for UpdatePolicy {
    fn default() -> Self {
        Self {
            on_start: true,
            interval: Some(Duration::from_secs(6 * 3600)),
        }
    }
}

impl UpdatePolicy {
    pub fn manual_only() -> Self {
        Self {
            on_start: false,
            interval: None,
        }
    }

    pub fn should_run_on_start(&self) -> bool {
        self.on_start
    }

    pub fn triggers(&self) -> Vec<UpdateTrigger> {
        let mut t = vec![UpdateTrigger::Manual];
        if self.on_start {
            t.push(UpdateTrigger::OnStart);
        }
        if self.interval.is_some() {
            t.push(UpdateTrigger::Interval);
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_interval() {
        let p = UpdatePolicy::default();
        assert!(p.should_run_on_start());
        assert!(p.triggers().contains(&UpdateTrigger::Interval));
    }
}
