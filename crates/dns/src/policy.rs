//! DNS policy surface.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveMode {
    System,
    Tunnel,
    FakeIp,
}

#[derive(Debug, Clone)]
pub struct DnsPolicy {
    pub mode: ResolveMode,
    pub prefer_doh: bool,
    pub cache_enabled: bool,
    pub default_ttl_secs: u32,
}

impl Default for DnsPolicy {
    fn default() -> Self {
        Self {
            mode: ResolveMode::Tunnel,
            prefer_doh: false,
            cache_enabled: true,
            default_ttl_secs: 60,
        }
    }
}
