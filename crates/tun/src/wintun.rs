//! Wintun session wiring with optional native packet IO.

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

/// Logical Wintun session; may hold a native session when `wintun-native` + DLL available.
pub struct WintunSession {
    request: WintunAdapterRequest,
    dll: WintunDllPath,
    state: WintunSessionState,
    capacity_ring: u32,
    native_active: bool,
    packets_in: u64,
    packets_out: u64,
    last_error: Option<String>,
    #[cfg(all(windows, feature = "wintun-native"))]
    native: Option<NativeBundle>,
}

#[cfg(all(windows, feature = "wintun-native"))]
struct NativeBundle {
    // Library must outlive session.
    _lib: netpilot_os_wintun::WintunLibrary,
    session: netpilot_os_wintun::WintunNativeSession,
}

impl std::fmt::Debug for WintunSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WintunSession")
            .field("state", &self.state)
            .field("adapter", &self.request.name)
            .field("native_active", &self.native_active)
            .field("packets_in", &self.packets_in)
            .field("packets_out", &self.packets_out)
            .finish()
    }
}

impl WintunSession {
    pub fn new(request: WintunAdapterRequest, dll: WintunDllPath) -> Self {
        Self {
            request,
            dll,
            state: WintunSessionState::Idle,
            capacity_ring: 0x200000, // 2 MiB default ring
            native_active: false,
            packets_in: 0,
            packets_out: 0,
            last_error: None,
            #[cfg(all(windows, feature = "wintun-native"))]
            native: None,
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

    pub fn is_native(&self) -> bool {
        self.native_active
    }

    pub fn stats(&self) -> (u64, u64) {
        (self.packets_in, self.packets_out)
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Attempt to load the library. Soft-fails to logical mode when DLL absent.
    pub fn load_library(&mut self) -> Result<(), TunError> {
        if self.request.name.is_empty() {
            return Err(TunError::InvalidConfig("adapter name empty"));
        }
        #[cfg(feature = "wintun-native")]
        {
            match &self.dll {
                WintunDllPath::BesideExecutable => {
                    match netpilot_os_wintun::load_first_available() {
                        Ok(lib) => {
                            let exports = lib.probe_exports();
                            if exports.is_empty() {
                                self.state = WintunSessionState::Failed;
                                self.last_error =
                                    Some("wintun.dll loaded but no known exports".into());
                                return Err(TunError::Io("wintun.dll loaded but no known exports"));
                            }
                            // Keep loaded only for probe path; open_session reloads for ownership.
                            drop(lib);
                            self.state = WintunSessionState::LibraryLoaded;
                        }
                        Err(e) => {
                            self.last_error = Some(e.to_string());
                            // Soft-fail: stay usable in mock mode when DLL absent.
                            self.state = WintunSessionState::LibraryLoaded;
                        }
                    }
                }
                WintunDllPath::Absolute(path) => {
                    match netpilot_os_wintun::load_from_path(std::path::Path::new(path)) {
                        Ok(_lib) => {
                            self.state = WintunSessionState::LibraryLoaded;
                        }
                        Err(e) => {
                            self.state = WintunSessionState::Failed;
                            self.last_error = Some(e.to_string());
                            return Err(TunError::Io(
                                "failed to load wintun.dll from absolute path",
                            ));
                        }
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(feature = "wintun-native"))]
        {
            self.state = WintunSessionState::LibraryLoaded;
            Ok(())
        }
    }

    pub fn create_adapter(&mut self) -> Result<(), TunError> {
        if self.state != WintunSessionState::LibraryLoaded {
            return Err(TunError::FailedPrecondition("library not loaded"));
        }
        self.state = WintunSessionState::AdapterCreated;
        Ok(())
    }

    /// Start session. On Windows with native feature, attempts real adapter.
    pub fn start_session(&mut self) -> Result<(), TunError> {
        if self.state != WintunSessionState::AdapterCreated {
            return Err(TunError::FailedPrecondition("adapter not created"));
        }
        #[cfg(all(windows, feature = "wintun-native"))]
        {
            let load = match &self.dll {
                WintunDllPath::BesideExecutable => netpilot_os_wintun::load_first_available(),
                WintunDllPath::Absolute(p) => {
                    netpilot_os_wintun::load_from_path(std::path::Path::new(p))
                }
            };
            match load {
                Ok(lib) => match lib.open_session(
                    &self.request.name,
                    &self.request.tunnel_type,
                    self.capacity_ring,
                ) {
                    Ok(session) => {
                        self.native = Some(NativeBundle {
                            _lib: lib,
                            session,
                        });
                        self.native_active = true;
                        self.state = WintunSessionState::SessionRunning;
                        return Ok(());
                    }
                    Err(e) => {
                        self.last_error = Some(e.to_string());
                        // Fall through to logical running for control-plane continuity.
                    }
                },
                Err(e) => {
                    self.last_error = Some(e.to_string());
                }
            }
        }
        self.native_active = false;
        self.state = WintunSessionState::SessionRunning;
        Ok(())
    }

    /// Receive one IP packet (native only). `None` on timeout / no data.
    pub fn receive_packet(&mut self, timeout_ms: u32) -> Result<Option<Vec<u8>>, TunError> {
        if self.state != WintunSessionState::SessionRunning {
            return Err(TunError::FailedPrecondition("session not running"));
        }
        #[cfg(all(windows, feature = "wintun-native"))]
        {
            if let Some(bundle) = self.native.as_mut() {
                match bundle.session.receive(timeout_ms) {
                    Ok(Some(pkt)) => {
                        self.packets_in = self.packets_in.saturating_add(1);
                        return Ok(Some(pkt));
                    }
                    Ok(None) => return Ok(None),
                    Err(e) => {
                        self.last_error = Some(e.to_string());
                        return Err(TunError::Io("wintun receive failed"));
                    }
                }
            }
        }
        let _ = timeout_ms;
        Ok(None)
    }

    /// Send one IP packet (native only).
    pub fn send_packet(&mut self, packet: &[u8]) -> Result<(), TunError> {
        if self.state != WintunSessionState::SessionRunning {
            return Err(TunError::FailedPrecondition("session not running"));
        }
        #[cfg(all(windows, feature = "wintun-native"))]
        {
            if let Some(bundle) = self.native.as_mut() {
                return match bundle.session.send(packet) {
                    Ok(()) => {
                        self.packets_out = self.packets_out.saturating_add(1);
                        Ok(())
                    }
                    Err(e) => {
                        self.last_error = Some(e.to_string());
                        Err(TunError::Io("wintun send failed"))
                    }
                };
            }
        }
        if packet.is_empty() {
            return Err(TunError::InvalidConfig("empty packet"));
        }
        // Logical mode: count only.
        self.packets_out = self.packets_out.saturating_add(1);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), TunError> {
        #[cfg(all(windows, feature = "wintun-native"))]
        {
            self.native = None;
        }
        self.native_active = false;
        self.state = WintunSessionState::Closed;
        Ok(())
    }

    pub fn ring_capacity(&self) -> u32 {
        self.capacity_ring
    }

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

/// Provider that prefers Wintun on Windows and falls back to logical semantics in CI.
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
        let mut session = p.open_session(TunConfig::default()).unwrap();
        assert_eq!(session.state(), WintunSessionState::SessionRunning);
        assert_eq!(session.as_tun_state(), TunState::Running);
        assert_eq!(p.opened_count(), 1);
        session.send_packet(&[0x45, 0, 0, 20]).unwrap();
        assert_eq!(session.stats().1, 1);
        session.stop().unwrap();
    }
}
