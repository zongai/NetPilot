//! IP/CIDR matching (NP-040).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpMatchError {
    InvalidIp(String),
    InvalidCidr(String),
}

impl std::fmt::Display for IpMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIp(s) => write!(f, "InvalidIp: {s}"),
            Self::InvalidCidr(s) => write!(f, "InvalidCidr: {s}"),
        }
    }
}

impl std::error::Error for IpMatchError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cidr {
    V4 { addr: Ipv4Addr, prefix: u8 },
    V6 { addr: Ipv6Addr, prefix: u8 },
}

pub fn parse_ip(s: &str) -> Result<IpAddr, IpMatchError> {
    s.trim()
        .parse()
        .map_err(|_| IpMatchError::InvalidIp(s.to_string()))
}

pub fn parse_cidr(s: &str) -> Result<Cidr, IpMatchError> {
    let s = s.trim();
    let (ip_s, prefix_s) = match s.split_once('/') {
        Some(pair) => pair,
        None => {
            // bare IP → full host prefix
            return match parse_ip(s)? {
                IpAddr::V4(addr) => Ok(Cidr::V4 { addr, prefix: 32 }),
                IpAddr::V6(addr) => Ok(Cidr::V6 { addr, prefix: 128 }),
            };
        }
    };
    let prefix: u8 = prefix_s
        .parse()
        .map_err(|_| IpMatchError::InvalidCidr(s.to_string()))?;
    match parse_ip(ip_s)? {
        IpAddr::V4(addr) => {
            if prefix > 32 {
                return Err(IpMatchError::InvalidCidr(s.to_string()));
            }
            Ok(Cidr::V4 { addr, prefix })
        }
        IpAddr::V6(addr) => {
            if prefix > 128 {
                return Err(IpMatchError::InvalidCidr(s.to_string()));
            }
            Ok(Cidr::V6 { addr, prefix })
        }
    }
}

fn ipv4_in_cidr(ip: Ipv4Addr, net: Ipv4Addr, prefix: u8) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = if prefix == 32 {
        u32::MAX
    } else {
        !((1u32 << (32 - prefix)) - 1)
    };
    (u32::from(ip) & mask) == (u32::from(net) & mask)
}

fn ipv6_in_cidr(ip: Ipv6Addr, net: Ipv6Addr, prefix: u8) -> bool {
    if prefix == 0 {
        return true;
    }
    let ip_bytes = ip.octets();
    let net_bytes = net.octets();
    let full_bytes = (prefix / 8) as usize;
    let rem_bits = prefix % 8;
    if ip_bytes[..full_bytes] != net_bytes[..full_bytes] {
        return false;
    }
    if rem_bits == 0 {
        return true;
    }
    let mask = !((1u8 << (8 - rem_bits)) - 1);
    (ip_bytes[full_bytes] & mask) == (net_bytes[full_bytes] & mask)
}

pub fn ip_in_cidr(ip: &str, cidr: &str) -> Result<bool, IpMatchError> {
    let addr = parse_ip(ip)?;
    let net = parse_cidr(cidr)?;
    Ok(match (addr, net) {
        (IpAddr::V4(ip), Cidr::V4 { addr, prefix }) => ipv4_in_cidr(ip, addr, prefix),
        (IpAddr::V6(ip), Cidr::V6 { addr, prefix }) => ipv6_in_cidr(ip, addr, prefix),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_cidr() {
        assert!(ip_in_cidr("10.1.2.3", "10.0.0.0/8").unwrap());
        assert!(!ip_in_cidr("11.0.0.1", "10.0.0.0/8").unwrap());
        assert!(ip_in_cidr("192.168.1.10", "192.168.1.0/24").unwrap());
        assert!(ip_in_cidr("1.2.3.4", "1.2.3.4").unwrap());
    }

    #[test]
    fn v6_cidr() {
        assert!(ip_in_cidr("2001:db8::1", "2001:db8::/32").unwrap());
        assert!(!ip_in_cidr("2001:db9::1", "2001:db8::/32").unwrap());
    }

    #[test]
    fn bad_input() {
        assert!(parse_ip("not-an-ip").is_err());
        assert!(parse_cidr("10.0.0.0/99").is_err());
    }
}
