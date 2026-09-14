//! UDP datagram parse/build.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
}

pub fn parse_udp(seg: &[u8]) -> Option<(UdpHeader, usize)> {
    if seg.len() < 8 {
        return None;
    }
    Some((
        UdpHeader {
            src_port: u16::from_be_bytes([seg[0], seg[1]]),
            dst_port: u16::from_be_bytes([seg[2], seg[3]]),
            length: u16::from_be_bytes([seg[4], seg[5]]),
        },
        8,
    ))
}

pub fn build_udp(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
    let len = (8 + payload.len()) as u16;
    let mut out = vec![0u8; 8 + payload.len()];
    out[0..2].copy_from_slice(&src_port.to_be_bytes());
    out[2..4].copy_from_slice(&dst_port.to_be_bytes());
    out[4..6].copy_from_slice(&len.to_be_bytes());
    out[8..].copy_from_slice(payload);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_build() {
        let d = build_udp(53, 53, b"dns");
        let (h, off) = parse_udp(&d).unwrap();
        assert_eq!(h.src_port, 53);
        assert_eq!(&d[off..], b"dns");
    }
}
