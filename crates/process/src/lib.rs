//! Process discovery and identity (NP-085..088 + Windows live resolver).

#![cfg_attr(windows, allow(unsafe_code))]
#![cfg_attr(not(windows), forbid(unsafe_code))]
#![allow(dead_code)]
#![allow(clippy::all)]

mod discovery;
mod identity;
mod lifecycle;
mod matcher;
mod path;

#[cfg(windows)]
mod win;

pub use discovery::{MockProcessTable, ProcessResolver, ProcessSnapshot};
pub use identity::ProcessIdentity;
pub use lifecycle::{PidEvent, PidTracker};
pub use matcher::{file_name, normalize_process_path, ProcessMatch, ProcessMatchKind};
pub use path::{normalize_exe_path, path_matches};

pub const CRATE_NAME: &str = "netpilot-process";

pub fn resolve_pid_live(pid: u32) -> Option<ProcessSnapshot> {
    #[cfg(windows)]
    {
        win::resolve_pid_path(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

pub fn list_processes_live() -> Vec<ProcessSnapshot> {
    #[cfg(windows)]
    {
        win::list_processes()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[derive(Debug, Default, Clone)]
pub struct LiveProcessResolver {
    mock: MockProcessTable,
    prefer_live: bool,
}

impl LiveProcessResolver {
    pub fn new() -> Self {
        Self {
            mock: MockProcessTable::new(),
            prefer_live: true,
        }
    }

    pub fn with_mock(table: MockProcessTable) -> Self {
        Self {
            mock: table,
            prefer_live: false,
        }
    }

    pub fn insert_mock(&mut self, snap: ProcessSnapshot) {
        self.mock.insert(snap);
    }

    pub fn resolve_pid(&self, pid: u32) -> Option<ProcessIdentity> {
        if self.prefer_live {
            if let Some(s) = resolve_pid_live(pid) {
                return Some(s.identity);
            }
        }
        self.mock.get(pid).map(|s| s.identity.clone())
    }

    pub fn resolve_name(&self, name: &str) -> Vec<(u32, ProcessIdentity)> {
        let mut out = Vec::new();
        if self.prefer_live {
            for snap in list_processes_live() {
                if snap.identity.matches_name(name) {
                    out.push((snap.pid, snap.identity));
                }
            }
        }
        if out.is_empty() {
            for (pid, id) in ProcessResolver::from_table(self.mock.clone()).resolve_name(name) {
                out.push((pid, id));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_resolve() {
        let mut table = MockProcessTable::new();
        table.insert(ProcessSnapshot {
            pid: 100,
            identity: ProcessIdentity::from_path(r"C:\a\chrome.exe"),
        });
        let r = LiveProcessResolver::with_mock(table);
        let id = r.resolve_pid(100).unwrap();
        assert_eq!(id.exe_name, "chrome.exe");
    }
}
