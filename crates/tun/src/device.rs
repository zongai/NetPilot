//! TUN device lifecycle and IP configuration (NP-063 / NP-064).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunState {
    Created,
    Configured,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

impl TunState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Configured => "configured",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TunConfig {
    pub name: String,
    pub mtu: u16,
    pub ipv4: Option<String>,
    pub prefix: u8,
}

impl Default for TunConfig {
    fn default() -> Self {
        Self {
            name: "NetPilot".into(),
            mtu: 1500,
            ipv4: None,
            prefix: 24,
        }
    }
}

impl TunConfig {
    pub fn validate(&self) -> Result<(), TunError> {
        if self.name.is_empty() || self.name.len() > 64 {
            return Err(TunError::InvalidConfig("name empty or too long"));
        }
        if self.mtu < 576 || self.mtu > 9000 {
            return Err(TunError::InvalidConfig("mtu out of range"));
        }
        if self.prefix > 32 {
            return Err(TunError::InvalidConfig("prefix > 32"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunError {
    InvalidConfig(&'static str),
    FailedPrecondition(&'static str),
    Io(&'static str),
    Unsupported(&'static str),
}

impl std::fmt::Display for TunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(m) => write!(f, "InvalidConfig: {m}"),
            Self::FailedPrecondition(m) => write!(f, "FailedPrecondition: {m}"),
            Self::Io(m) => write!(f, "Io: {m}"),
            Self::Unsupported(m) => write!(f, "Unsupported: {m}"),
        }
    }
}

impl std::error::Error for TunError {}

/// Logical TUN device handle (mock or future Wintun).
#[derive(Debug)]
pub struct TunDevice {
    config: TunConfig,
    state: TunState,
    mock: bool,
}

impl TunDevice {
    pub(crate) fn new_mock(config: TunConfig) -> Self {
        Self {
            config,
            state: TunState::Created,
            mock: true,
        }
    }

    pub fn state(&self) -> TunState {
        self.state
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn mtu(&self) -> u16 {
        self.config.mtu
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }

    /// Assign IPv4 address/prefix on the virtual interface (NP-064).
    pub fn configure_ip(&mut self, ipv4: String, prefix: u8) -> Result<(), TunError> {
        if matches!(self.state, TunState::Running | TunState::Starting) {
            return Err(TunError::FailedPrecondition(
                "cannot reconfigure while running",
            ));
        }
        if ipv4.trim().is_empty() || prefix > 32 {
            return Err(TunError::InvalidConfig("bad ipv4/prefix"));
        }
        self.config.ipv4 = Some(ipv4);
        self.config.prefix = prefix;
        self.state = TunState::Configured;
        Ok(())
    }

    pub fn ipv4(&self) -> Option<&str> {
        self.config.ipv4.as_deref()
    }

    pub fn start(&mut self) -> Result<(), TunError> {
        match self.state {
            TunState::Created | TunState::Configured | TunState::Stopped => {
                self.state = TunState::Starting;
                // Mock: immediate ready. Wintun would create session here.
                self.state = TunState::Running;
                Ok(())
            }
            other => Err(TunError::FailedPrecondition(other.as_str())),
        }
    }

    pub fn stop(&mut self) -> Result<(), TunError> {
        match self.state {
            TunState::Running | TunState::Starting => {
                self.state = TunState::Stopping;
                self.state = TunState::Stopped;
                Ok(())
            }
            TunState::Stopped => Ok(()),
            other => Err(TunError::FailedPrecondition(other.as_str())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_then_start() {
        let mut d = TunDevice::new_mock(TunConfig::default());
        d.configure_ip("10.8.0.2".into(), 24).unwrap();
        assert_eq!(d.ipv4(), Some("10.8.0.2"));
        d.start().unwrap();
        assert_eq!(d.state(), TunState::Running);
        assert!(d.configure_ip("10.8.0.3".into(), 24).is_err());
    }
}
