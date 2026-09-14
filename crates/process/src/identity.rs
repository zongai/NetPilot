//! Stable process identity model (NP-086).

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessIdentity {
    /// Basename of the executable (lowercase preferred).
    pub exe_name: String,
    /// Normalized full path when known.
    pub exe_path: String,
    /// Stable hash/token for grouping (path-based; no secrets).
    pub uid_hash: String,
}

impl ProcessIdentity {
    pub fn from_path(path: &str) -> Self {
        let normalized = crate::path::normalize_exe_path(path);
        let exe_name = normalized
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let uid_hash = simple_hash(&normalized);
        Self {
            exe_name,
            exe_path: normalized,
            uid_hash,
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.exe_name.eq_ignore_ascii_case(name.trim())
    }
}

fn simple_hash(s: &str) -> String {
    // Non-crypto stable fingerprint for identity grouping.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_path_stable() {
        let a = ProcessIdentity::from_path(r"C:\App\foo.EXE");
        let b = ProcessIdentity::from_path(r"c:/app/foo.exe");
        assert_eq!(a.exe_name, "foo.exe");
        assert_eq!(a.uid_hash, b.uid_hash);
    }
}
