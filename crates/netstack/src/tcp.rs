//! Minimal TCP segment parse/build and connection state.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub data_off: u8,
    pub flags: u8,
    pub window: u16,
}

pub const FLAG_FIN: u8 = 0x01;
pub const FLAG_SYN: u8 = 0x02;
pub const FLAG_RST: u8 = 0x04;
pub const FLAG_PSH: u8 = 0x08;
pub const FLAG_ACK: u8 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    SynReceived,
    Established,
    FinWait,
    ClosedWait,
}

pub fn parse_tcp(seg: &[u8]) -> Option<(TcpHeader, usize)> {
    if seg.len() < 20 {
        return None;
    }
    let data_off = ((seg[12] >> 4) as usize) * 4;
    if data_off < 20 || seg.len() < data_off {
        return None;
    }
    Some((
        TcpHeader {
            src_port: u16::from_be_bytes([seg[0], seg[1]]),
            dst_port: u16::from_be_bytes([seg[2], seg[3]]),
            seq: u32::from_be_bytes([seg[4], seg[5], seg[6], seg[7]]),
            ack: u32::from_be_bytes([seg[8], seg[9], seg[10], seg[11]]),
            data_off: (data_off / 4) as u8,
            flags: seg[13],
            window: u16::from_be_bytes([seg[14], seg[15]]),
        },
        data_off,
    ))
}

pub fn build_tcp(
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    window: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut out = vec![0u8; 20 + payload.len()];
    out[0..2].copy_from_slice(&src_port.to_be_bytes());
    out[2..4].copy_from_slice(&dst_port.to_be_bytes());
    out[4..8].copy_from_slice(&seq.to_be_bytes());
    out[8..12].copy_from_slice(&ack.to_be_bytes());
    out[12] = 5 << 4; // data offset 5 (20 bytes)
    out[13] = flags;
    out[14..16].copy_from_slice(&window.to_be_bytes());
    // checksum left 0 (optional for some stacks); compute if needed by caller
    out[20..].copy_from_slice(payload);
    out
}

pub fn tcp_checksum(src: [u8; 4], dst: [u8; 4], tcp: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    sum += u16::from_be_bytes([src[0], src[1]]) as u32;
    sum += u16::from_be_bytes([src[2], src[3]]) as u32;
    sum += u16::from_be_bytes([dst[0], dst[1]]) as u32;
    sum += u16::from_be_bytes([dst[2], dst[3]]) as u32;
    sum += 6u32; // protocol
    sum += tcp.len() as u32;
    let mut i = 0;
    while i + 1 < tcp.len() {
        sum += u16::from_be_bytes([tcp[i], tcp[i + 1]]) as u32;
        i += 2;
    }
    if i < tcp.len() {
        sum += (tcp[i] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub fn set_tcp_checksum(src: [u8; 4], dst: [u8; 4], tcp: &mut [u8]) {
    tcp[16] = 0;
    tcp[17] = 0;
    let c = tcp_checksum(src, dst, tcp);
    tcp[16..18].copy_from_slice(&c.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_syn() {
        let seg = build_tcp(1234, 80, 1, 0, FLAG_SYN, 65535, &[]);
        let (h, _) = parse_tcp(&seg).unwrap();
        assert_eq!(h.flags & FLAG_SYN, FLAG_SYN);
        assert_eq!(h.dst_port, 80);
    }
}
