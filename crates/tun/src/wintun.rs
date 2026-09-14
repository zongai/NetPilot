//! Wintun session wiring surface (formal build).
//!
//! Real `wintun.dll` FFI is feature-gated and not linked in default CI.
//! This module defines the load path, adapter request, and session lifecycle
//! that a future `wintun-ffi` backend will implement.

use crate::device::{TunConfig, TunError, TunState};

/// Where to look for `wintun.dll` on Windows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WintunDllPath {
    /// Same directory as the Core executable.
    #[default]
    BesideExecutable,
    /// Explicit filesystem path.
    Absolute(String),
}

#[derive(Debug, Clone)]
pub struct WintunAdapterRequest {
    pub name: String,
    pub tunnel_type: String,
    pub requested_guid: Option<String>,
}

impl Default for WintunAdapterRequest {
    fn default() -> Self {
        Self {
            name: "NetPilot".into(),
            tunnel_type: "NetPilot".into(),
            requested_guid: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WintunSessionState {
    Idle,
    LibraryLoaded,
    AdapterCreated,
    SessionRunning,
    Closed,
    Failed,
}

/// Logical Wintun session handle (no native resources in default builds).
#[derive(Debug)]
pub struct WintunSession {
    request: WintunAdapterRequest,
    dll: WintunDllPath,
    state: WintunSessionState,
    capacity_ring: u32,
}

impl WintunSession {
    pub fn new(request: WintunAdapterRequest, dll: WintunDllPath) -> Self {
        Self {
            request,
            dll,
            state: WintunSessionState::Idle,
            capacity_ring: 0x200000, // 2 MiB default ring (Wintun-style)
        }
    }

    pub fn from_tun_config(config: &TunConfig) -> Self {
        Self::new(
            WintunAdapterRequest {
                name: config.name.clone(),
                tunnel_type: "NetPilot".into(),
                requested_guid: None,
            },
            WintunDllPath::default(),
        )
    }

    pub fn state(&self) -> WintunSessionState {
        self.state
    }

    pub fn adapter_name(&self) -> &str {
        &self.request.name
    }

    pub fn dll_path(&self) -> &WintunDllPath {
        &self.dll
    }

    /// Attempt to load the library. Default builds only validate configuration.
    pub fn load_library(&mut self) -> Result<(), TunError> {
        if self.request.name.is_empty() {
            return Err(TunError::InvalidConfig("adapter name empty"));
        }
        // Native LoadLibraryW reserved for feature `wintun-native` (Windows).
        self.state = WintunSessionState::LibraryLoaded;
        Ok(())
    }

    pub fn create_adapter(&mut self) -> Result<(), TunError> {
        if self.state != WintunSessionState::LibraryLoaded {
            return Err(TunError::FailedPrecondition("library not loaded"));
        }
        self.state = WintunSessionState::AdapterCreated;
        Ok(())
    }

    pub fn start_session(&mut self) -> Result<(), TunError> {
        if self.state != WintunSessionState::AdapterCreated {
            return Err(TunError::FailedPrecondition("adapter not created"));
        }
        self.state = WintunSessionState::SessionRunning;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), TunError> {
        self.state = WintunSessionState::Closed;
        Ok(())
    }

    pub fn ring_capacity(&self) -> u32 {
        self.capacity_ring
    }

    /// Map session state to high-level TUN state for Core.
    pub fn as_tun_state(&self) -> TunState {
        match self.state {
            WintunSessionState::Idle | WintunSessionState::LibraryLoaded => TunState::Created,
            WintunSessionState::AdapterCreated => TunState::Configured,
            WintunSessionState::SessionRunning => TunState::Running,
            WintunSessionState::Closed => TunState::Stopped,
            WintunSessionState::Failed => TunState::Failed,
        }
    }
}

/// Provider that prefers Wintun on Windows and falls back to mock semantics in CI.
#[derive(Debug, Default)]
pub struct WintunTunProvider {
    opened: u32,
}

impl WintunTunProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn opened_count(&self) -> u32 {
        self.opened
    }

    pub fn open_session(&mut self, config: TunConfig) -> Result<WintunSession, TunError> {
        config.validate()?;
        let mut session = WintunSession::from_tun_config(&config);
        session.load_library()?;
        session.create_adapter()?;
        session.start_session()?;
        self.opened = self.opened.saturating_add(1);
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_lifecycle_without_native() {
        let mut p = WintunTunProvider::new();
        let session = p.open_session(TunConfig::default()).unwrap();
        assert_eq!(session.state(), WintunSessionState::SessionRunning);
        assert_eq!(session.as_tun_state(), TunState::Running);
        assert_eq!(p.opened_count(), 1);
    }
}
