//! IPv4 header parse/build.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Header {
    pub ihl: u8,
    pub total_len: u16,
    pub id: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub src: [u8; 4],
    pub dst: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpError {
    Truncated,
    NotV4,
    BadHeader,
}

impl std::fmt::Display for IpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IpError {}

pub fn parse_ipv4(packet: &[u8]) -> Result<(Ipv4Header, usize), IpError> {
    if packet.len() < 20 {
        return Err(IpError::Truncated);
    }
    if packet[0] >> 4 != 4 {
        return Err(IpError::NotV4);
    }
    let ihl = (packet[0] & 0x0f) as usize * 4;
    if ihl < 20 || packet.len() < ihl {
        return Err(IpError::BadHeader);
    }
    let total_len = u16::from_be_bytes([packet[2], packet[3]]);
    Ok((
        Ipv4Header {
            ihl: (ihl / 4) as u8,
            total_len,
            id: u16::from_be_bytes([packet[4], packet[5]]),
            ttl: packet[8],
            protocol: packet[9],
            src: [packet[12], packet[13], packet[14], packet[15]],
            dst: [packet[16], packet[17], packet[18], packet[19]],
        },
        ihl,
    ))
}

pub fn addr_str(a: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", a[0], a[1], a[2], a[3])
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn build_ipv4(
    src: [u8; 4],
    dst: [u8; 4],
    protocol: u8,
    payload: &[u8],
    id: u16,
) -> Vec<u8> {
    let total = 20 + payload.len();
    let mut out = vec![0u8; total];
    out[0] = 0x45;
    out[2..4].copy_from_slice(&(total as u16).to_be_bytes());
    out[4..6].copy_from_slice(&id.to_be_bytes());
    out[8] = 64;
    out[9] = protocol;
    out[12..16].copy_from_slice(&src);
    out[16..20].copy_from_slice(&dst);
    let csum = checksum(&out[..20]);
    out[10..12].copy_from_slice(&csum.to_be_bytes());
    out[20..].copy_from_slice(payload);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let p = build_ipv4([10, 0, 0, 1], [10, 0, 0, 2], 6, b"hi", 1);
        let (h, ihl) = parse_ipv4(&p).unwrap();
        assert_eq!(ihl, 20);
        assert_eq!(h.protocol, 6);
        assert_eq!(&p[20..], b"hi");
    }
}
