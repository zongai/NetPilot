//! TUN provider abstraction (NP-061).

use crate::device::{TunConfig, TunDevice, TunError};

/// Factory for platform TUN devices.
pub trait TunProvider {
    fn name(&self) -> &str;
    fn open(&mut self, config: TunConfig) -> Result<TunDevice, TunError>;
}

/// In-memory provider for tests and non-Windows hosts.
#[derive(Debug, Default)]
pub struct MockTunProvider {
    opened: u32,
}

impl MockTunProvider {
    pub fn new() -> Self {
        Self { opened: 0 }
    }

    pub fn opened_count(&self) -> u32 {
        self.opened
    }
}

impl TunProvider for MockTunProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn open(&mut self, config: TunConfig) -> Result<TunDevice, TunError> {
        config.validate()?;
        self.opened = self.opened.saturating_add(1);
        Ok(TunDevice::new_mock(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_increments() {
        let mut p = MockTunProvider::new();
        let _ = p.open(TunConfig::default()).unwrap();
        assert_eq!(p.opened_count(), 1);
    }
}
