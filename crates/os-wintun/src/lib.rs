//! Optional dynamic loader for `wintun.dll`.
//!
//! Does not link against Wintun at build time. On non-Windows targets every
//! call returns [`WintunLoadError::Unsupported`].

#![cfg_attr(not(windows), allow(dead_code))]
#![allow(unsafe_code)] // Win32 LoadLibrary / GetProcAddress only

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WintunLoadError {
    Unsupported,
    NotFound(String),
    Api(String),
    InvalidPath,
}

impl std::fmt::Display for WintunLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Unsupported"),
            Self::NotFound(p) => write!(f, "NotFound({p})"),
            Self::Api(m) => write!(f, "Api({m})"),
            Self::InvalidPath => write!(f, "InvalidPath"),
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
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> RawHandle;
        fn FreeLibrary(hModule: RawHandle) -> i32;
        fn GetProcAddress(hModule: RawHandle, lpProcName: *const u8) -> *const core::ffi::c_void;
        fn GetLastError() -> u32;
    }

    const INVALID: RawHandle = 0 as RawHandle;

    fn wide(path: &Path) -> Result<Vec<u16>, WintunLoadError> {
        let s = path.to_str().ok_or(WintunLoadError::InvalidPath)?;
        Ok(OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect())
    }

    /// Loaded module handle; freed on drop.
    pub struct WintunLibrary {
        handle: OwnedHandle,
        path: PathBuf,
    }

    impl WintunLibrary {
        pub fn path(&self) -> &Path {
            &self.path
        }

        /// True if a known export name resolves (best-effort API presence check).
        pub fn has_export(&self, name: &str) -> bool {
            let mut cname = name.as_bytes().to_vec();
            cname.push(0);
            let p = unsafe { GetProcAddress(self.handle.as_raw_handle(), cname.as_ptr()) };
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
            ];
            NAMES
                .iter()
                .copied()
                .filter(|n| self.has_export(n))
                .collect()
        }
    }

    impl Drop for WintunLibrary {
        fn drop(&mut self) {
            unsafe {
                let _ = FreeLibrary(self.handle.as_raw_handle());
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
        Ok(WintunLibrary {
            handle: unsafe { OwnedHandle::from_raw_handle(h) },
            path: path.to_path_buf(),
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
pub use win::{load_first_available, load_from_path, WintunLibrary};

#[cfg(not(windows))]
pub struct WintunLibrary;

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
