//! Fake-IP model and allocator (NP-080).

use std::collections::HashMap;

use crate::resolver::DnsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeIpMapping {
    pub domain: String,
    pub fake_ip: String,
}

/// Allocates addresses from a reserved pool (default 198.18.0.0/16).
#[derive(Debug)]
pub struct FakeIpAllocator {
    base: [u8; 4],
    /// Host bits available (e.g. /16 → 16 bits).
    host_bits: u8,
    next: u32,
    domain_to_ip: HashMap<String, String>,
    ip_to_domain: HashMap<String, String>,
}

impl FakeIpAllocator {
    pub fn new(base_ip: &str, prefix: u8) -> Self {
        let mut base = [198, 18, 0, 0];
        if let Ok(parts) = parse_ipv4(base_ip) {
            base = parts;
        }
        let host_bits = 32u8.saturating_sub(prefix.min(32));
        Self {
            base,
            host_bits,
            next: 1, // skip network address
            domain_to_ip: HashMap::new(),
            ip_to_domain: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.domain_to_ip.len()
    }

    pub fn is_empty(&self) -> bool {
        self.domain_to_ip.is_empty()
    }

    pub fn allocate(&mut self, domain: &str) -> Result<String, DnsError> {
        let key = domain.to_ascii_lowercase();
        if let Some(ip) = self.domain_to_ip.get(&key) {
            return Ok(ip.clone());
        }
        let max = if self.host_bits >= 32 {
            u32::MAX
        } else {
            (1u32 << self.host_bits).saturating_sub(1)
        };
        if self.next > max || self.next == 0 {
            return Err(DnsError::Exhausted("fake-ip pool exhausted"));
        }
        let host = self.next;
        self.next = self.next.saturating_add(1);
        let ip = host_to_ipv4(self.base, host, self.host_bits);
        self.domain_to_ip.insert(key.clone(), ip.clone());
        self.ip_to_domain.insert(ip.clone(), key);
        Ok(ip)
    }

    pub fn lookup_domain(&self, fake_ip: &str) -> Option<&str> {
        self.ip_to_domain.get(fake_ip).map(|s| s.as_str())
    }

    pub fn lookup_ip(&self, domain: &str) -> Option<&str> {
        self.domain_to_ip
            .get(&domain.to_ascii_lowercase())
            .map(|s| s.as_str())
    }

    pub fn contains_ip(&self, ip: &str) -> bool {
        self.ip_to_domain.contains_key(ip)
    }
}

fn parse_ipv4(s: &str) -> Result<[u8; 4], ()> {
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() != 4 {
        return Err(());
    }
    let mut out = [0u8; 4];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.parse().map_err(|_| ())?;
    }
    Ok(out)
}

fn host_to_ipv4(base: [u8; 4], host: u32, host_bits: u8) -> String {
    let base_u = u32::from_be_bytes(base);
    let mask = if host_bits >= 32 {
        0
    } else {
        !((1u32 << host_bits) - 1)
    };
    let ip = (base_u & mask) | (host & !mask);
    let b = ip.to_be_bytes();
    format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_stable() {
        let mut a = FakeIpAllocator::new("198.18.0.0", 16);
        let ip1 = a.allocate("foo.example").unwrap();
        let ip2 = a.allocate("foo.example").unwrap();
        assert_eq!(ip1, ip2);
        assert_eq!(a.lookup_domain(&ip1), Some("foo.example"));
    }
}
