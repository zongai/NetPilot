//! IPC timeout and cancellation helpers (NP-021).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cooperative cancel flag shared across accept/connect/request paths.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    inner: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), CancelError> {
        if self.is_cancelled() {
            Err(CancelError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelError {
    Cancelled,
}

impl std::fmt::Display for CancelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cancelled")
    }
}

impl std::error::Error for CancelError {}

/// Deadline helper for request handling budgets.
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    start: Instant,
    budget: Duration,
}

impl Deadline {
    pub fn after(budget: Duration) -> Self {
        Self {
            start: Instant::now(),
            budget,
        }
    }

    pub fn remaining(&self) -> Duration {
        self.budget.saturating_sub(self.start.elapsed())
    }

    pub fn is_expired(&self) -> bool {
        self.start.elapsed() >= self.budget
    }

    pub fn check(&self) -> Result<(), TimeoutError> {
        if self.is_expired() {
            Err(TimeoutError {
                budget: self.budget,
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError {
    pub budget: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timeout: exceeded {:?}", self.budget)
    }
}

impl std::error::Error for TimeoutError {}

/// Combine cancel + deadline checks.
pub fn check_budget(token: &CancelToken, deadline: &Deadline) -> Result<(), BudgetError> {
    token.check().map_err(|_| BudgetError::Cancelled)?;
    deadline.check().map_err(|e| BudgetError::Timeout {
        budget: e.budget,
    })?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetError {
    Cancelled,
    Timeout { budget: Duration },
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Timeout { budget } => write!(f, "Timeout: exceeded {budget:?}"),
        }
    }
}

impl std::error::Error for BudgetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_token() {
        let t = CancelToken::new();
        t.check().unwrap();
        t.cancel();
        assert!(matches!(t.check(), Err(CancelError::Cancelled)));
    }

    #[test]
    fn deadline_expires() {
        let d = Deadline::after(Duration::from_millis(0));
        assert!(d.is_expired());
        assert!(d.check().is_err());
    }

    #[test]
    fn budget_prefers_cancel() {
        let t = CancelToken::new();
        t.cancel();
        let d = Deadline::after(Duration::from_secs(10));
        assert!(matches!(
            check_budget(&t, &d),
            Err(BudgetError::Cancelled)
        ));
    }
}
