//! Single Core instance guard (NP-150).
//!
//! File-based lock under the runtime data directory. Cross-platform; no secrets.

use crate::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const LOCK_FILE_NAME: &str = "core.instance.lock";

/// Holds an exclusive instance lock for the process lifetime.
#[derive(Debug)]
pub struct InstanceLock {
    path: PathBuf,
    _file: File,
    pid: u32,
}

impl InstanceLock {
    /// Try to acquire the lock at `dir/core.instance.lock`.
    ///
    /// If another live process owns the lock, returns FailedPrecondition.
    pub fn try_acquire(dir: &Path) -> Result<Self, Error> {
        fs::create_dir_all(dir).map_err(|_| Error::Internal("create instance dir failed"))?;
        let path = dir.join(LOCK_FILE_NAME);

        if path.exists() {
            if let Ok(mut f) = File::open(&path) {
                let mut buf = String::new();
                let _ = f.read_to_string(&mut buf);
                if let Ok(old_pid) = buf.trim().parse::<u32>() {
                    if pid_seems_alive(old_pid) {
                        return Err(Error::FailedPrecondition {
                            from: crate::RuntimeState::Created,
                            to: crate::RuntimeState::Starting,
                            message: "another core instance holds the lock",
                        });
                    }
                }
            }
            let _ = fs::remove_file(&path);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| Error::Internal("open instance lock failed"))?;

        let pid = std::process::id();
        write!(file, "{pid}").map_err(|_| Error::Internal("write instance lock failed"))?;
        file.sync_all()
            .map_err(|_| Error::Internal("sync instance lock failed"))?;

        Ok(Self {
            path,
            _file: file,
            pid,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn pid_seems_alive(pid: u32) -> bool {
    // Prefer /proc (Linux). Other Unix: treat unknown as stale lock.
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(windows)]
fn pid_seems_alive(pid: u32) -> bool {
    // Without Win32 here (forbid unsafe in this crate), treat lock as stale unless
    // the pid matches this process. Desktop/Core launcher enforces single instance
    // more strongly on Windows later.
    pid == std::process::id()
}

#[cfg(not(any(unix, windows)))]
fn pid_seems_alive(_pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn acquire_and_release() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("netpilot-instance-{stamp}"));
        let lock = InstanceLock::try_acquire(&dir).expect("first acquire");
        assert!(lock.path().exists());
        drop(lock);
        assert!(!dir.join(LOCK_FILE_NAME).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_acquire_same_process_allowed_after_drop() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("netpilot-instance2-{stamp}"));
        {
            let _a = InstanceLock::try_acquire(&dir).unwrap();
        }
        let _b = InstanceLock::try_acquire(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
