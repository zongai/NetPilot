//! Process discovery and identity (NP-085…NP-088).

#![forbid(unsafe_code)]

mod discovery;
mod identity;
mod lifecycle;
mod path;

pub use discovery::{MockProcessTable, ProcessResolver, ProcessSnapshot};
pub use identity::ProcessIdentity;
pub use lifecycle::{PidEvent, PidTracker};
pub use path::{normalize_exe_path, path_matches};

pub const CRATE_NAME: &str = "netpilot-process";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_and_track() {
        let mut table = MockProcessTable::new();
        table.insert(ProcessSnapshot {
            pid: 100,
            identity: ProcessIdentity {
                exe_name: "chrome.exe".into(),
                exe_path: r"C:\Program Files\Google\Chrome\Application\chrome.exe".into(),
                uid_hash: "h1".into(),
            },
        });
        let resolver = ProcessResolver::from_table(table);
        let id = resolver.resolve_pid(100).unwrap();
        assert_eq!(id.exe_name, "chrome.exe");
        assert!(path_matches(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"*\chrome.exe"
        ));

        let mut tracker = PidTracker::new();
        tracker.observe(100, id.clone());
        assert!(tracker.get(100).is_some());
        tracker.note_exit(100);
        assert!(tracker.get(100).is_none());
    }
}
