//! Executable path attribution (NP-087).

/// Normalize path for comparison: lowercase, `\` → `/`, trim trailing slash.
pub fn normalize_exe_path(path: &str) -> String {
    let mut s = path.trim().replace('\\', "/").to_ascii_lowercase();
    while s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    s
}

/// Glob-like match: `*` matches any substring; otherwise case-insensitive equality of normalized paths.
pub fn path_matches(actual: &str, pattern: &str) -> bool {
    let a = normalize_exe_path(actual);
    let p = normalize_exe_path(pattern);
    if p == "*" {
        return true;
    }
    if !p.contains('*') {
        return a == p;
    }
    // Simple single/multi star substring matching.
    let parts: Vec<&str> = p.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    let mut rest = a.as_str();
    if !parts[0].is_empty() {
        if !rest.starts_with(parts[0]) {
            return false;
        }
        rest = &rest[parts[0].len()..];
    }
    for (i, part) in parts.iter().enumerate().skip(1) {
        if part.is_empty() {
            if i == parts.len() - 1 {
                return true;
            }
            continue;
        }
        if let Some(idx) = rest.find(part) {
            rest = &rest[idx + part.len()..];
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_glob() {
        assert_eq!(normalize_exe_path(r"C:\Foo\Bar.EXE"), "c:/foo/bar.exe");
        assert!(path_matches(r"C:\Program Files\App\app.exe", r"*/app.exe"));
        assert!(!path_matches(r"C:\Other\x.exe", r"*/app.exe"));
    }
}
