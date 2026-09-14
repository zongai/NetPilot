//! DNS transports: UDP, TCP, DoT, DoH (NP-075…NP-078).

use crate::resolver::{DnsError, DnsQuery, DnsResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Udp,
    Tcp,
    DoT,
    DoH,
    Mock,
}

impl TransportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Udp => "udp",
            Self::Tcp => "tcp",
            Self::DoT => "dot",
            Self::DoH => "doh",
            Self::Mock => "mock",
        }
    }
}

pub trait DnsTransport {
    fn kind(&self) -> TransportKind;
    fn query(&mut self, q: &DnsQuery) -> Result<DnsResponse, DnsError>;
}

/// UDP/53 transport skeleton (no socket in unit tests).
#[derive(Debug, Clone)]
pub struct UdpDnsTransport {
    pub server: String,
    pub timeout_ms: u32,
}

impl Default for UdpDnsTransport {
    fn default() -> Self {
        Self {
            server: "1.1.1.1:53".into(),
            timeout_ms: 3_000,
        }
    }
}

impl DnsTransport for UdpDnsTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Udp
    }

    fn query(&mut self, q: &DnsQuery) -> Result<DnsResponse, DnsError> {
        // Real UDP dial deferred; surface clear error when not mocked.
        Err(DnsError::Transport(format!(
            "udp transport not connected (server={})",
            self.server
        )))
        .map_err(|e| {
            let _ = q;
            e
        })
    }
}

/// TCP/53 transport skeleton.
#[derive(Debug, Clone)]
pub struct TcpDnsTransport {
    pub server: String,
    pub timeout_ms: u32,
}

impl Default for TcpDnsTransport {
    fn default() -> Self {
        Self {
            server: "1.1.1.1:53".into(),
            timeout_ms: 5_000,
        }
    }
}

impl DnsTransport for TcpDnsTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Tcp
    }

    fn query(&mut self, q: &DnsQuery) -> Result<DnsResponse, DnsError> {
        let _ = q;
        Err(DnsError::Transport(format!(
            "tcp transport not connected (server={})",
            self.server
        )))
    }
}

/// DNS-over-TLS (853) skeleton.
#[derive(Debug, Clone)]
pub struct DoTTransport {
    pub server: String,
    pub sni: String,
    pub timeout_ms: u32,
}

impl Default for DoTTransport {
    fn default() -> Self {
        Self {
            server: "1.1.1.1:853".into(),
            sni: "cloudflare-dns.com".into(),
            timeout_ms: 5_000,
        }
    }
}

impl DnsTransport for DoTTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::DoT
    }

    fn query(&mut self, q: &DnsQuery) -> Result<DnsResponse, DnsError> {
        let _ = q;
        Err(DnsError::Transport(format!(
            "dot transport not connected (sni={})",
            self.sni
        )))
    }
}

/// DNS-over-HTTPS skeleton.
#[derive(Debug, Clone)]
pub struct DohTransport {
    pub url: String,
    pub timeout_ms: u32,
}

impl Default for DohTransport {
    fn default() -> Self {
        Self {
            url: "https://cloudflare-dns.com/dns-query".into(),
            timeout_ms: 5_000,
        }
    }
}

impl DnsTransport for DohTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::DoH
    }

    fn query(&mut self, q: &DnsQuery) -> Result<DnsResponse, DnsError> {
        let _ = q;
        Err(DnsError::Transport(format!(
            "doh transport not connected (url={})",
            self.url
        )))
    }
}

/// In-memory transport for tests.
#[derive(Debug, Default)]
pub struct MockTransport {
    records: std::collections::HashMap<String, Vec<String>>,
    hits: u64,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&mut self, name: &str, addr: &str) {
        self.records
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push(addr.to_string());
    }

    pub fn hits(&self) -> u64 {
        self.hits
    }
}

impl DnsTransport for MockTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Mock
    }

    fn query(&mut self, q: &DnsQuery) -> Result<DnsResponse, DnsError> {
        self.hits = self.hits.saturating_add(1);
        let key = q.name.to_ascii_lowercase();
        match self.records.get(&key) {
            Some(addrs) if !addrs.is_empty() => Ok(DnsResponse {
                name: q.name.clone(),
                addresses: addrs.clone(),
                ttl_secs: 60,
                from_cache: false,
                transport: TransportKind::Mock,
            }),
            _ => Err(DnsError::NxDomain(q.name.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_seed_and_query() {
        let mut t = MockTransport::new();
        t.seed("a.test", "1.2.3.4");
        let r = t.query(&DnsQuery::a("a.test")).unwrap();
        assert_eq!(r.addresses, vec!["1.2.3.4".to_string()]);
    }

    #[test]
    fn kinds() {
        assert_eq!(UdpDnsTransport::default().kind(), TransportKind::Udp);
        assert_eq!(TcpDnsTransport::default().kind(), TransportKind::Tcp);
        assert_eq!(DoTTransport::default().kind(), TransportKind::DoT);
        assert_eq!(DohTransport::default().kind(), TransportKind::DoH);
    }
}
