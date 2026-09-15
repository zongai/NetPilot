#![allow(dead_code)]
#![allow(clippy::all)]
//! Dynamic loader + full Wintun FFI surface (`wintun.dll`).
//!
//! Does not link against Wintun at build time. On non-Windows targets every
//! call returns [`WintunLoadError::Unsupported`].

#![cfg_attr(not(windows), allow(dead_code))]
#![allow(unsafe_code)] // Win32 LoadLibrary / GetProcAddress / Wintun FFI

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WintunLoadError {
    Unsupported,
    NotFound(String),
    Api(String),
    InvalidPath,
    MissingExport(&'static str),
}

impl std::fmt::Display for WintunLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Unsupported"),
            Self::NotFound(p) => write!(f, "NotFound({p})"),
            Self::Api(m) => write!(f, "Api({m})"),
            Self::InvalidPath => write!(f, "InvalidPath"),
            Self::MissingExport(n) => write!(f, "MissingExport({n})"),
        }
    }
}

impl std::error::Error for WintunLoadError {}

/// Candidate paths for `wintun.dll` next to the executable and CWD.
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("wintun.dll"));
        }
    }
    out.push(PathBuf::from("wintun.dll"));
    out
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::RawHandle;
    use std::ptr;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> RawHandle;
        fn FreeLibrary(hModule: RawHandle) -> i32;
        fn GetProcAddress(hModule: RawHandle, lpProcName: *const u8) -> *const core::ffi::c_void;
        fn GetLastError() -> u32;
        fn WaitForSingleObject(hHandle: RawHandle, dwMilliseconds: u32) -> u32;
    }

    const INVALID: RawHandle = 0 as RawHandle;
    pub const WAIT_OBJECT_0: u32 = 0;
    pub const WAIT_TIMEOUT: u32 = 0x0000_0102;

    fn wide(path: &Path) -> Result<Vec<u16>, WintunLoadError> {
        let s = path.to_str().ok_or(WintunLoadError::InvalidPath)?;
        Ok(OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect())
    }

    fn wide_str(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    type FnCreateAdapter = unsafe extern "C" fn(
        name: *const u16,
        tunnel_type: *const u16,
        requested_guid: *const u8,
    ) -> *mut core::ffi::c_void;
    type FnOpenAdapter = unsafe extern "C" fn(name: *const u16) -> *mut core::ffi::c_void;
    type FnCloseAdapter = unsafe extern "C" fn(adapter: *mut core::ffi::c_void);
    type FnGetAdapterLuid =
        unsafe extern "C" fn(adapter: *mut core::ffi::c_void, luid: *mut u64) -> i32;
    type FnStartSession = unsafe extern "C" fn(
        adapter: *mut core::ffi::c_void,
        capacity: u32,
    ) -> *mut core::ffi::c_void;
    type FnEndSession = unsafe extern "C" fn(session: *mut core::ffi::c_void);
    type FnGetReadWaitEvent = unsafe extern "C" fn(session: *mut core::ffi::c_void) -> RawHandle;
    type FnReceivePacket =
        unsafe extern "C" fn(session: *mut core::ffi::c_void, packet_size: *mut u32) -> *mut u8;
    type FnReleaseReceivePacket =
        unsafe extern "C" fn(session: *mut core::ffi::c_void, packet: *const u8);
    type FnAllocateSendPacket =
        unsafe extern "C" fn(session: *mut core::ffi::c_void, packet_size: u32) -> *mut u8;
    type FnSendPacket = unsafe extern "C" fn(session: *mut core::ffi::c_void, packet: *const u8);

    struct Api {
        create_adapter: FnCreateAdapter,
        open_adapter: FnOpenAdapter,
        close_adapter: FnCloseAdapter,
        get_adapter_luid: FnGetAdapterLuid,
        start_session: FnStartSession,
        end_session: FnEndSession,
        get_read_wait_event: FnGetReadWaitEvent,
        receive_packet: FnReceivePacket,
        release_receive_packet: FnReleaseReceivePacket,
        allocate_send_packet: FnAllocateSendPacket,
        send_packet: FnSendPacket,
    }

    unsafe fn load_fn<T>(module: RawHandle, name: &'static str) -> Result<T, WintunLoadError> {
        let mut cname = name.as_bytes().to_vec();
        cname.push(0);
        let p = GetProcAddress(module, cname.as_ptr());
        if p.is_null() {
            return Err(WintunLoadError::MissingExport(name));
        }
        Ok(std::mem::transmute_copy(&p))
    }

    impl Api {
        unsafe fn from_module(module: RawHandle) -> Result<Self, WintunLoadError> {
            Ok(Self {
                create_adapter: load_fn(module, "WintunCreateAdapter")?,
                open_adapter: load_fn(module, "WintunOpenAdapter")?,
                close_adapter: load_fn(module, "WintunCloseAdapter")?,
                get_adapter_luid: load_fn(module, "WintunGetAdapterLUID")?,
                start_session: load_fn(module, "WintunStartSession")?,
                end_session: load_fn(module, "WintunEndSession")?,
                get_read_wait_event: load_fn(module, "WintunGetReadWaitEvent")?,
                receive_packet: load_fn(module, "WintunReceivePacket")?,
                release_receive_packet: load_fn(module, "WintunReleaseReceivePacket")?,
                allocate_send_packet: load_fn(module, "WintunAllocateSendPacket")?,
                send_packet: load_fn(module, "WintunSendPacket")?,
            })
        }
    }

    /// Loaded module + resolved Wintun API.
    pub struct WintunLibrary {
        handle: RawHandle,
        path: PathBuf,
        api: Api,
    }

    // HMODULE is process-local; intended single-thread owner with cross-thread handoff.
    unsafe impl Send for WintunLibrary {}

    impl WintunLibrary {
        pub fn path(&self) -> &Path {
            &self.path
        }

        pub fn has_export(&self, name: &str) -> bool {
            let mut cname = name.as_bytes().to_vec();
            cname.push(0);
            let p = unsafe { GetProcAddress(self.handle, cname.as_ptr()) };
            !p.is_null()
        }

        pub fn probe_exports(&self) -> Vec<&'static str> {
            const NAMES: &[&str] = &[
                "WintunCreateAdapter",
                "WintunOpenAdapter",
                "WintunCloseAdapter",
                "WintunStartSession",
                "WintunEndSession",
                "WintunGetAdapterLUID",
                "WintunGetReadWaitEvent",
                "WintunReceivePacket",
                "WintunReleaseReceivePacket",
                "WintunAllocateSendPacket",
                "WintunSendPacket",
            ];
            NAMES
                .iter()
                .copied()
                .filter(|n| self.has_export(n))
                .collect()
        }

        /// Create or open adapter and start a session with the given ring capacity.
        pub fn open_session(
            &self,
            name: &str,
            tunnel_type: &str,
            capacity: u32,
        ) -> Result<WintunNativeSession, WintunLoadError> {
            let wname = wide_str(name);
            let wtype = wide_str(tunnel_type);
            let adapter =
                unsafe { (self.api.create_adapter)(wname.as_ptr(), wtype.as_ptr(), ptr::null()) };
            if adapter.is_null() {
                // Try open existing
                let opened = unsafe { (self.api.open_adapter)(wname.as_ptr()) };
                if opened.is_null() {
                    let err = unsafe { GetLastError() };
                    return Err(WintunLoadError::Api(format!(
                        "WintunCreate/OpenAdapter failed winerr={err}"
                    )));
                }
                return self.start_on_adapter(opened, capacity, true);
            }
            self.start_on_adapter(adapter, capacity, true)
        }

        fn start_on_adapter(
            &self,
            adapter: *mut core::ffi::c_void,
            capacity: u32,
            owns_adapter: bool,
        ) -> Result<WintunNativeSession, WintunLoadError> {
            let mut luid: u64 = 0;
            unsafe {
                let _ = (self.api.get_adapter_luid)(adapter, &mut luid);
            }
            let session = unsafe { (self.api.start_session)(adapter, capacity.max(0x20000)) };
            if session.is_null() {
                if owns_adapter {
                    unsafe { (self.api.close_adapter)(adapter) };
                }
                let err = unsafe { GetLastError() };
                return Err(WintunLoadError::Api(format!(
                    "WintunStartSession failed winerr={err}"
                )));
            }
            let wait_event = unsafe { (self.api.get_read_wait_event)(session) };
            Ok(WintunNativeSession {
                adapter,
                session,
                wait_event,
                luid,
                owns_adapter,
                // Function pointers copied so session can operate without holding library lifetime
                // in a complex borrow; library must outlive session in practice.
                close_adapter: self.api.close_adapter,
                end_session: self.api.end_session,
                receive_packet: self.api.receive_packet,
                release_receive_packet: self.api.release_receive_packet,
                allocate_send_packet: self.api.allocate_send_packet,
                send_packet: self.api.send_packet,
            })
        }
    }

    impl Drop for WintunLibrary {
        fn drop(&mut self) {
            unsafe {
                if !self.handle.is_null() {
                    let _ = FreeLibrary(self.handle);
                }
            }
        }
    }

    /// Live Wintun adapter + session with packet IO.
    pub struct WintunNativeSession {
        adapter: *mut core::ffi::c_void,
        session: *mut core::ffi::c_void,
        wait_event: RawHandle,
        luid: u64,
        owns_adapter: bool,
        close_adapter: FnCloseAdapter,
        end_session: FnEndSession,
        receive_packet: FnReceivePacket,
        release_receive_packet: FnReleaseReceivePacket,
        allocate_send_packet: FnAllocateSendPacket,
        send_packet: FnSendPacket,
    }

    // Wintun handles are process-local; send across threads is OK for single-owner use.
    unsafe impl Send for WintunNativeSession {}

    impl WintunNativeSession {
        pub fn luid(&self) -> u64 {
            self.luid
        }

        /// Block up to `timeout_ms` for a packet; returns owned bytes.
        pub fn receive(&mut self, timeout_ms: u32) -> Result<Option<Vec<u8>>, WintunLoadError> {
            // Non-blocking try first
            let mut size: u32 = 0;
            let pkt = unsafe { (self.receive_packet)(self.session, &mut size) };
            if !pkt.is_null() {
                let slice = unsafe { std::slice::from_raw_parts(pkt, size as usize) };
                let owned = slice.to_vec();
                unsafe { (self.release_receive_packet)(self.session, pkt) };
                return Ok(Some(owned));
            }
            if timeout_ms == 0 {
                return Ok(None);
            }
            if self.wait_event.is_null() {
                return Ok(None);
            }
            let wait = unsafe { WaitForSingleObject(self.wait_event, timeout_ms) };
            if wait == WAIT_TIMEOUT {
                return Ok(None);
            }
            if wait != WAIT_OBJECT_0 {
                return Ok(None);
            }
            size = 0;
            let pkt = unsafe { (self.receive_packet)(self.session, &mut size) };
            if pkt.is_null() {
                return Ok(None);
            }
            let slice = unsafe { std::slice::from_raw_parts(pkt, size as usize) };
            let owned = slice.to_vec();
            unsafe { (self.release_receive_packet)(self.session, pkt) };
            Ok(Some(owned))
        }

        pub fn send(&mut self, packet: &[u8]) -> Result<(), WintunLoadError> {
            if packet.is_empty() || packet.len() > 0xffff {
                return Err(WintunLoadError::Api("invalid packet size".into()));
            }
            let buf = unsafe { (self.allocate_send_packet)(self.session, packet.len() as u32) };
            if buf.is_null() {
                let err = unsafe { GetLastError() };
                return Err(WintunLoadError::Api(format!(
                    "WintunAllocateSendPacket failed winerr={err}"
                )));
            }
            unsafe {
                ptr::copy_nonoverlapping(packet.as_ptr(), buf, packet.len());
                (self.send_packet)(self.session, buf);
            }
            Ok(())
        }
    }

    impl Drop for WintunNativeSession {
        fn drop(&mut self) {
            unsafe {
                if !self.session.is_null() {
                    (self.end_session)(self.session);
                    self.session = ptr::null_mut();
                }
                if self.owns_adapter && !self.adapter.is_null() {
                    (self.close_adapter)(self.adapter);
                    self.adapter = ptr::null_mut();
                }
            }
        }
    }

    pub fn load_from_path(path: &Path) -> Result<WintunLibrary, WintunLoadError> {
        if !path.exists() {
            return Err(WintunLoadError::NotFound(path.display().to_string()));
        }
        let w = wide(path)?;
        let h = unsafe { LoadLibraryW(w.as_ptr()) };
        if h == INVALID || h.is_null() {
            let err = unsafe { GetLastError() };
            return Err(WintunLoadError::Api(format!(
                "LoadLibraryW failed path={} winerr={err}",
                path.display()
            )));
        }
        let api = unsafe { Api::from_module(h) }?;
        Ok(WintunLibrary {
            handle: h,
            path: path.to_path_buf(),
            api,
        })
    }

    pub fn load_first_available() -> Result<WintunLibrary, WintunLoadError> {
        let mut last = WintunLoadError::NotFound("wintun.dll".into());
        for p in candidate_paths() {
            match load_from_path(&p) {
                Ok(lib) => return Ok(lib),
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

#[cfg(windows)]
pub use win::{load_first_available, load_from_path, WintunLibrary, WintunNativeSession};

#[cfg(not(windows))]
pub struct WintunLibrary {
    path: PathBuf,
}

#[cfg(not(windows))]
impl WintunLibrary {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn has_export(&self, _name: &str) -> bool {
        false
    }

    pub fn probe_exports(&self) -> Vec<&'static str> {
        Vec::new()
    }

    pub fn open_session(
        &self,
        _name: &str,
        _tunnel_type: &str,
        _capacity: u32,
    ) -> Result<WintunNativeSession, WintunLoadError> {
        Err(WintunLoadError::Unsupported)
    }
}

#[cfg(not(windows))]
pub struct WintunNativeSession;

#[cfg(not(windows))]
impl WintunNativeSession {
    pub fn luid(&self) -> u64 {
        0
    }
    pub fn receive(&mut self, _timeout_ms: u32) -> Result<Option<Vec<u8>>, WintunLoadError> {
        Err(WintunLoadError::Unsupported)
    }
    pub fn send(&mut self, _packet: &[u8]) -> Result<(), WintunLoadError> {
        Err(WintunLoadError::Unsupported)
    }
}

#[cfg(not(windows))]
pub fn load_from_path(_path: &Path) -> Result<WintunLibrary, WintunLoadError> {
    Err(WintunLoadError::Unsupported)
}

#[cfg(not(windows))]
pub fn load_first_available() -> Result<WintunLibrary, WintunLoadError> {
    Err(WintunLoadError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_non_empty() {
        assert!(!candidate_paths().is_empty());
    }

    #[test]
    #[cfg(not(windows))]
    fn unsupported_off_windows() {
        assert!(matches!(
            load_first_available(),
            Err(WintunLoadError::Unsupported)
        ));
    }
}
