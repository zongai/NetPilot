//! SOCKS-style address encoding shared by SOCKS5 / Trojan / VLESS.

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::OutboundError;

#[derive(Debug, Clone)]
pub enum TargetAddr {
    Domain(String, u16),
    V4(Ipv4Addr, u16),
    V6(Ipv6Addr, u16),
}

impl TargetAddr {
    pub fn from_host_port(host: &str, port: u16) -> Self {
        if let Ok(v4) = host.parse::<Ipv4Addr>() {
            Self::V4(v4, port)
        } else if let Ok(v6) = host.parse::<Ipv6Addr>() {
            Self::V6(v6, port)
        } else {
            Self::Domain(host.to_string(), port)
        }
    }
}

/// Encode ATYP + ADDR + PORT (SOCKS5 / Trojan request body style).
pub fn encode_socks_addr(addr: &TargetAddr) -> Result<Vec<u8>, OutboundError> {
    let mut out = Vec::with_capacity(32);
    match addr {
        TargetAddr::V4(ip, port) => {
            out.push(0x01);
            out.extend_from_slice(&ip.octets());
            out.extend_from_slice(&port.to_be_bytes());
        }
        TargetAddr::V6(ip, port) => {
            out.push(0x04);
            out.extend_from_slice(&ip.octets());
            out.extend_from_slice(&port.to_be_bytes());
        }
        TargetAddr::Domain(host, port) => {
            let b = host.as_bytes();
            if b.len() > 255 {
                return Err(OutboundError::Invalid("domain too long".into()));
            }
            out.push(0x03);
            out.push(b.len() as u8);
            out.extend_from_slice(b);
            out.extend_from_slice(&port.to_be_bytes());
        }
    }
    Ok(out)
}
