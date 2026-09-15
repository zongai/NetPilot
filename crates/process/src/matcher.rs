//! Process path matcher for rules (NP-197).

use std::path::{Path, PathBuf};

/// Match a process image path against rule patterns (suffix / exact / contains).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessMatchKind {
    Exact,
    Suffix,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMatch {
    pub kind: ProcessMatchKind,
    pub pattern: String,
}

impl ProcessMatch {
    pub fn matches(&self, image_path: &str) -> bool {
        let path = image_path.replace('/', "\\").to_ascii_lowercase();
        let pat = self.pattern.replace('/', "\\").to_ascii_lowercase();
        match self.kind {
            ProcessMatchKind::Exact => path == pat,
            ProcessMatchKind::Suffix => path.ends_with(&pat),
            ProcessMatchKind::Contains => path.contains(&pat),
        }
    }
}

/// Normalize Windows-style process path for comparison.
pub fn normalize_process_path(p: &str) -> PathBuf {
    PathBuf::from(p.replace('/', "\\"))
}

pub fn file_name(path: &str) -> Option<&str> {
    Path::new(path).file_name().and_then(|s| s.to_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_exe() {
        let m = ProcessMatch {
            kind: ProcessMatchKind::Suffix,
            pattern: r"\chrome.exe".into(),
        };
        assert!(m.matches(r"C:\Program Files\Google\Chrome\Application\chrome.exe"));
        assert!(!m.matches(r"C:\edge.exe"));
    }

    #[test]
    fn contains() {
        let m = ProcessMatch {
            kind: ProcessMatchKind::Contains,
            pattern: "steam".into(),
        };
        assert!(m.matches(r"D:\Games\Steam\steam.exe"));
    }
}
