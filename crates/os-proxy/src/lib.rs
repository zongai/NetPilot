//! System proxy control for Windows (user Internet Settings + optional WinHTTP).
//!
//! Enables NetPilot to act as the OS default proxy so browsers and WinINET apps
//! route through the local HTTP CONNECT / SOCKS inbound.

#![cfg_attr(not(windows), allow(dead_code))]
#![cfg_attr(windows, allow(unsafe_code))]

#![allow(dead_code)]
#![allow(clippy::all)]
#![allow(clippy::upper_case_acronyms)]

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyError {
    Unsupported,
    Api(String),
    Invalid(String),
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Unsupported"),
            Self::Api(m) => write!(f, "Api({m})"),
            Self::Invalid(m) => write!(f, "Invalid({m})"),
        }
    }
}

impl std::error::Error for ProxyError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemProxySettings {
    pub enabled: bool,
    /// e.g. `127.0.0.1:7890` or `socks=127.0.0.1:1080`
    pub server: String,
    /// Bypass list, e.g. `localhost;127.*;<local>`
    pub bypass: String,
}

impl Default for SystemProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            server: String::new(),
            bypass: "localhost;127.*;<local>".into(),
        }
    }
}

impl SystemProxySettings {
    pub fn http(host: &str, port: u16) -> Self {
        Self {
            enabled: true,
            server: format!("{host}:{port}"),
            bypass: "localhost;127.*;<local>".into(),
        }
    }

    pub fn socks(host: &str, port: u16) -> Self {
        Self {
            enabled: true,
            server: format!("socks={host}:{port}"),
            bypass: "localhost;127.*;<local>".into(),
        }
    }
}

/// Snapshot saved before apply, for rollback.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SavedProxyState {
    pub enabled: bool,
    pub server: String,
    pub bypass: String,
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    type HKEY = *mut core::ffi::c_void;
    type DWORD = u32;
    type LONG = i32;
    type LSTATUS = LONG;

    const HKEY_CURRENT_USER: HKEY = 0x80000001u32 as HKEY;
    const KEY_READ: DWORD = 0x20019;
    const KEY_WRITE: DWORD = 0x20006;
    const KEY_READ_WRITE: DWORD = KEY_READ | KEY_WRITE;
    const REG_SZ: DWORD = 1;
    const REG_DWORD: DWORD = 4;
    const ERROR_SUCCESS: LSTATUS = 0;
    const ERROR_FILE_NOT_FOUND: LSTATUS = 2;

    const INTERNET_OPTION_SETTINGS_CHANGED: DWORD = 39;
    const INTERNET_OPTION_REFRESH: DWORD = 37;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            hkey: HKEY,
            sub: *const u16,
            options: DWORD,
            sam: DWORD,
            result: *mut HKEY,
        ) -> LSTATUS;
        fn RegCloseKey(hkey: HKEY) -> LSTATUS;
        fn RegSetValueExW(
            hkey: HKEY,
            name: *const u16,
            reserved: DWORD,
            ty: DWORD,
            data: *const u8,
            len: DWORD,
        ) -> LSTATUS;
        fn RegQueryValueExW(
            hkey: HKEY,
            name: *const u16,
            reserved: *mut DWORD,
            ty: *mut DWORD,
            data: *mut u8,
            len: *mut DWORD,
        ) -> LSTATUS;
        fn RegDeleteValueW(hkey: HKEY, name: *const u16) -> LSTATUS;
    }

    #[link(name = "wininet")]
    extern "system" {
        fn InternetSetOptionW(
            hinternet: *mut core::ffi::c_void,
            option: DWORD,
            buffer: *mut core::ffi::c_void,
            len: DWORD,
        ) -> i32;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    const SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    fn open_key(write: bool) -> Result<HKEY, ProxyError> {
        let sub = wide(SUBKEY);
        let mut h: HKEY = ptr::null_mut();
        let sam = if write { KEY_READ_WRITE } else { KEY_READ };
        let st = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, sam, &mut h) };
        if st != ERROR_SUCCESS {
            return Err(ProxyError::Api(format!("RegOpenKeyExW={st}")));
        }
        Ok(h)
    }

    fn set_dword(h: HKEY, name: &str, value: u32) -> Result<(), ProxyError> {
        let n = wide(name);
        let bytes = value.to_le_bytes();
        let st = unsafe { RegSetValueExW(h, n.as_ptr(), 0, REG_DWORD, bytes.as_ptr(), 4) };
        if st != ERROR_SUCCESS {
            return Err(ProxyError::Api(format!("RegSetValueEx DWORD {name}={st}")));
        }
        Ok(())
    }

    fn set_string(h: HKEY, name: &str, value: &str) -> Result<(), ProxyError> {
        let n = wide(name);
        let data = wide(value);
        let bytes =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };
        let st = unsafe {
            RegSetValueExW(
                h,
                n.as_ptr(),
                0,
                REG_SZ,
                bytes.as_ptr(),
                (data.len() * 2) as DWORD,
            )
        };
        if st != ERROR_SUCCESS {
            return Err(ProxyError::Api(format!("RegSetValueEx SZ {name}={st}")));
        }
        Ok(())
    }

    fn query_dword(h: HKEY, name: &str) -> Result<Option<u32>, ProxyError> {
        let n = wide(name);
        let mut ty: DWORD = 0;
        let mut data = [0u8; 4];
        let mut len: DWORD = 4;
        let st = unsafe {
            RegQueryValueExW(
                h,
                n.as_ptr(),
                ptr::null_mut(),
                &mut ty,
                data.as_mut_ptr(),
                &mut len,
            )
        };
        if st == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if st != ERROR_SUCCESS {
            return Err(ProxyError::Api(format!("RegQuery DWORD {name}={st}")));
        }
        Ok(Some(u32::from_le_bytes(data)))
    }

    fn query_string(h: HKEY, name: &str) -> Result<Option<String>, ProxyError> {
        let n = wide(name);
        let mut ty: DWORD = 0;
        let mut len: DWORD = 0;
        let st = unsafe {
            RegQueryValueExW(
                h,
                n.as_ptr(),
                ptr::null_mut(),
                &mut ty,
                ptr::null_mut(),
                &mut len,
            )
        };
        if st == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if st != ERROR_SUCCESS && len == 0 {
            return Err(ProxyError::Api(format!("RegQuery SZ size {name}={st}")));
        }
        let mut buf = vec![0u8; len as usize];
        let st = unsafe {
            RegQueryValueExW(
                h,
                n.as_ptr(),
                ptr::null_mut(),
                &mut ty,
                buf.as_mut_ptr(),
                &mut len,
            )
        };
        if st != ERROR_SUCCESS {
            return Err(ProxyError::Api(format!("RegQuery SZ {name}={st}")));
        }
        // UTF-16 LE
        let u16s: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .filter(|c| *c != 0)
            .collect();
        Ok(Some(String::from_utf16_lossy(&u16s)))
    }

    fn notify_settings_changed() {
        unsafe {
            let _ = InternetSetOptionW(
                ptr::null_mut(),
                INTERNET_OPTION_SETTINGS_CHANGED,
                ptr::null_mut(),
                0,
            );
            let _ =
                InternetSetOptionW(ptr::null_mut(), INTERNET_OPTION_REFRESH, ptr::null_mut(), 0);
        }
    }

    pub fn query() -> Result<SystemProxySettings, ProxyError> {
        let h = open_key(false)?;
        let enabled = query_dword(h, "ProxyEnable")?.unwrap_or(0) != 0;
        let server = query_string(h, "ProxyServer")?.unwrap_or_default();
        let bypass = query_string(h, "ProxyOverride")?.unwrap_or_default();
        unsafe {
            RegCloseKey(h);
        }
        Ok(SystemProxySettings {
            enabled,
            server,
            bypass,
        })
    }

    pub fn apply(settings: &SystemProxySettings) -> Result<SavedProxyState, ProxyError> {
        if settings.enabled && settings.server.trim().is_empty() {
            return Err(ProxyError::Invalid("server required when enabling".into()));
        }
        let prev = match query() {
            Ok(q) => SavedProxyState {
                enabled: q.enabled,
                server: q.server,
                bypass: q.bypass,
            },
            Err(_) => SavedProxyState::default(),
        };
        let h = open_key(true)?;
        set_dword(h, "ProxyEnable", if settings.enabled { 1 } else { 0 })?;
        if settings.enabled {
            set_string(h, "ProxyServer", &settings.server)?;
            let bypass = if settings.bypass.is_empty() {
                "localhost;127.*;<local>"
            } else {
                settings.bypass.as_str()
            };
            set_string(h, "ProxyOverride", bypass)?;
        }
        unsafe {
            RegCloseKey(h);
        }
        notify_settings_changed();
        Ok(prev)
    }

    pub fn restore(saved: &SavedProxyState) -> Result<(), ProxyError> {
        let h = open_key(true)?;
        set_dword(h, "ProxyEnable", if saved.enabled { 1 } else { 0 })?;
        if !saved.server.is_empty() {
            set_string(h, "ProxyServer", &saved.server)?;
        }
        if !saved.bypass.is_empty() {
            set_string(h, "ProxyOverride", &saved.bypass)?;
        }
        unsafe {
            RegCloseKey(h);
        }
        notify_settings_changed();
        Ok(())
    }

    pub fn disable() -> Result<SavedProxyState, ProxyError> {
        apply(&SystemProxySettings {
            enabled: false,
            server: String::new(),
            bypass: String::new(),
        })
    }
}

#[cfg(windows)]
pub use win::{
    apply as apply_system_proxy, disable as disable_system_proxy, query as query_system_proxy,
    restore as restore_system_proxy,
};

#[cfg(not(windows))]
pub fn query_system_proxy() -> Result<SystemProxySettings, ProxyError> {
    Ok(SystemProxySettings::default())
}

#[cfg(not(windows))]
pub fn apply_system_proxy(settings: &SystemProxySettings) -> Result<SavedProxyState, ProxyError> {
    let _ = settings;
    Ok(SavedProxyState::default())
}

#[cfg(not(windows))]
pub fn restore_system_proxy(saved: &SavedProxyState) -> Result<(), ProxyError> {
    let _ = saved;
    Ok(())
}

#[cfg(not(windows))]
pub fn disable_system_proxy() -> Result<SavedProxyState, ProxyError> {
    Ok(SavedProxyState::default())
}

/// Auto-switch controller: apply system proxy when local inbound starts, restore on stop.
#[derive(Debug, Default)]
pub struct SystemProxyAuto {
    saved: Option<SavedProxyState>,
    active: bool,
    mode: AutoProxyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoProxyMode {
    #[default]
    Off,
    Socks,
    Http,
}

impl SystemProxyAuto {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn mode(&self) -> AutoProxyMode {
        self.mode
    }

    /// Enable system proxy pointing at local SOCKS or HTTP inbound.
    pub fn enable_local(
        &mut self,
        mode: AutoProxyMode,
        host: &str,
        port: u16,
    ) -> Result<(), ProxyError> {
        if matches!(mode, AutoProxyMode::Off) {
            return self.disable();
        }
        let settings = match mode {
            AutoProxyMode::Socks => SystemProxySettings::socks(host, port),
            AutoProxyMode::Http => SystemProxySettings::http(host, port),
            AutoProxyMode::Off => return Ok(()),
        };
        let saved = apply_system_proxy(&settings)?;
        if self.saved.is_none() {
            self.saved = Some(saved);
        }
        self.active = true;
        self.mode = mode;
        Ok(())
    }

    /// Restore previous system proxy settings.
    pub fn disable(&mut self) -> Result<(), ProxyError> {
        if let Some(ref saved) = self.saved {
            restore_system_proxy(saved)?;
        } else if self.active {
            let _ = disable_system_proxy();
        }
        self.saved = None;
        self.active = false;
        self.mode = AutoProxyMode::Off;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders() {
        let h = SystemProxySettings::http("127.0.0.1", 7890);
        assert!(h.enabled);
        assert_eq!(h.server, "127.0.0.1:7890");
        let s = SystemProxySettings::socks("127.0.0.1", 1080);
        assert!(s.server.starts_with("socks="));
    }

    #[test]
    fn query_and_noop_apply_off() {
        let q = query_system_proxy().unwrap();
        let _ = q;
        let _ = disable_system_proxy();
    }

    #[test]
    fn auto_controller_toggle() {
        let mut auto = SystemProxyAuto::new();
        // On non-Windows this is a no-op success path.
        let _ = auto.enable_local(AutoProxyMode::Socks, "127.0.0.1", 1080);
        let _ = auto.disable();
        assert!(!auto.is_active());
    }
}
