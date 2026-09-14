//! Domain matching (NP-039).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainMatchKind {
    Exact,
    Suffix,
    Keyword,
}

/// Match host against pattern. Host should already be lowercased by caller when possible.
pub fn domain_matches(kind: DomainMatchKind, pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if pattern.is_empty() || host.is_empty() {
        return false;
    }
    match kind {
        DomainMatchKind::Exact => host == pattern,
        DomainMatchKind::Suffix => host == pattern || host.ends_with(&format!(".{pattern}")),
        DomainMatchKind::Keyword => host.contains(&pattern),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact() {
        assert!(domain_matches(
            DomainMatchKind::Exact,
            "example.com",
            "example.com"
        ));
        assert!(!domain_matches(
            DomainMatchKind::Exact,
            "example.com",
            "www.example.com"
        ));
    }

    #[test]
    fn suffix() {
        assert!(domain_matches(
            DomainMatchKind::Suffix,
            "example.com",
            "www.example.com"
        ));
        assert!(domain_matches(
            DomainMatchKind::Suffix,
            "example.com",
            "example.com"
        ));
        assert!(!domain_matches(
            DomainMatchKind::Suffix,
            "example.com",
            "notexample.com"
        ));
    }

    #[test]
    fn keyword() {
        assert!(domain_matches(
            DomainMatchKind::Keyword,
            "ads",
            "ads.foo.com",
        ));
        assert!(!domain_matches(
            DomainMatchKind::Keyword,
            "ads",
            "example.com",
        ));
    }
}
