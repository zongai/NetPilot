//! Windows system resolver integration surface (NP-074).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemResolverInfo {
    pub available: bool,
    pub notes: Vec<&'static str>,
}

/// Placeholder for OS resolver (GetAddrInfo / DnsQuery_W on Windows).
#[derive(Debug, Default)]
pub struct SystemResolver;

impl SystemResolver {
    pub fn probe() -> SystemResolverInfo {
        let mut notes = vec![
            "System resolver will call platform APIs (Windows: DnsQuery_W / GetAddrInfoEx)",
            "Used when DnsPolicy.mode == System or as bootstrap for DoH endpoints",
        ];
        let available = cfg!(windows);
        if available {
            notes.push("Windows target: system resolver available at runtime");
        } else {
            notes.push("Non-Windows CI: system resolver marked unavailable");
        }
        SystemResolverInfo { available, notes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_has_notes() {
        let info = SystemResolver::probe();
        assert!(!info.notes.is_empty());
    }
}
