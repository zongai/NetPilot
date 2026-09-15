//! Userspace stack: ingest TUN IPv4 packets → TCP state → route decision hooks.

use std::time::Instant;

use crate::conn::{ConnTable, FourTuple, TcpConn};
use crate::ipv4::{addr_str, build_ipv4, parse_ipv4};
use crate::tcp::{
    build_tcp, parse_tcp, set_tcp_checksum, TcpState, FLAG_ACK, FLAG_FIN, FLAG_RST, FLAG_SYN,
};
use crate::udp::parse_udp;

#[derive(Debug, Clone)]
pub struct StackEvent {
    pub kind: StackEventKind,
    pub src: String,
    pub dst: String,
    pub sport: u16,
    pub dport: u16,
    pub outbound_hint: Option<String>,
    pub tuple: Option<FourTuple>,
    /// Application payload (TCP data) when present.
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackEventKind {
    TcpSyn,
    TcpData,
    TcpFin,
    TcpRst,
    UdpDatagram,
    Other,
}

#[derive(Debug, Default)]
pub struct NetStack {
    conns: ConnTable,
    next_id: u16,
    pub packets_in: u64,
    pub packets_out: u64,
    pub syns: u64,
    pub replies: u64,
}

impl NetStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn conn_count(&self) -> usize {
        self.conns.len()
    }

    pub fn conns_mut(&mut self) -> &mut ConnTable {
        &mut self.conns
    }

    /// Process one IPv4 packet from TUN. Returns reply packets + event for the engine.
    pub fn handle_inbound(
        &mut self,
        packet: &[u8],
        outbound_name: &str,
    ) -> (Vec<Vec<u8>>, Option<StackEvent>) {
        self.packets_in = self.packets_in.saturating_add(1);
        let Ok((ip, ihl)) = parse_ipv4(packet) else {
            return (Vec::new(), None);
        };
        let payload = &packet[ihl..];
        match ip.protocol {
            6 => self.handle_tcp(ip.src, ip.dst, payload, outbound_name),
            17 => self.handle_udp(ip.src, ip.dst, payload),
            _ => (Vec::new(), None),
        }
    }

    fn handle_tcp(
        &mut self,
        src: [u8; 4],
        dst: [u8; 4],
        seg: &[u8],
        outbound_name: &str,
    ) -> (Vec<Vec<u8>>, Option<StackEvent>) {
        let Some((hdr, off)) = parse_tcp(seg) else {
            return (Vec::new(), None);
        };
        let key = FourTuple {
            src,
            dst,
            sport: hdr.src_port,
            dport: hdr.dst_port,
        };
        let data = &seg[off..];

        if hdr.flags & FLAG_SYN != 0 && hdr.flags & FLAG_ACK == 0 {
            self.syns = self.syns.saturating_add(1);
            let isn = 0x1000_0000u32.wrapping_add(self.next_id as u32);
            self.next_id = self.next_id.wrapping_add(1);
            let mut tcp = build_tcp(
                hdr.dst_port,
                hdr.src_port,
                isn,
                hdr.seq.wrapping_add(1),
                FLAG_SYN | FLAG_ACK,
                65535,
                &[],
            );
            set_tcp_checksum(dst, src, &mut tcp);
            let reply = build_ipv4(dst, src, 6, &tcp, self.next_id);
            self.packets_out = self.packets_out.saturating_add(1);
            self.replies = self.replies.saturating_add(1);
            self.conns.insert(
                key.clone(),
                TcpConn {
                    state: TcpState::SynReceived,
                    client_seq: hdr.seq.wrapping_add(1),
                    server_seq: isn.wrapping_add(1),
                    outbound_name: outbound_name.to_string(),
                    created: Instant::now(),
                    bytes_up: 0,
                    bytes_down: 0,
                },
            );
            return (
                vec![reply],
                Some(StackEvent {
                    kind: StackEventKind::TcpSyn,
                    src: addr_str(src),
                    dst: addr_str(dst),
                    sport: hdr.src_port,
                    dport: hdr.dst_port,
                    outbound_hint: Some(outbound_name.to_string()),
                    tuple: Some(key),
                    payload: Vec::new(),
                }),
            );
        }

        if let Some(conn) = self.conns.get_mut(&key) {
            if hdr.flags & FLAG_ACK != 0 && conn.state == TcpState::SynReceived {
                conn.state = TcpState::Established;
            }
            if !data.is_empty() && conn.state == TcpState::Established {
                conn.bytes_up = conn.bytes_up.saturating_add(data.len() as u64);
                conn.client_seq = hdr.seq.wrapping_add(data.len() as u32);
                let mut tcp = build_tcp(
                    hdr.dst_port,
                    hdr.src_port,
                    conn.server_seq,
                    conn.client_seq,
                    FLAG_ACK,
                    65535,
                    &[],
                );
                set_tcp_checksum(dst, src, &mut tcp);
                let reply = build_ipv4(dst, src, 6, &tcp, self.next_id);
                self.next_id = self.next_id.wrapping_add(1);
                self.packets_out = self.packets_out.saturating_add(1);
                let name = conn.outbound_name.clone();
                return (
                    vec![reply],
                    Some(StackEvent {
                        kind: StackEventKind::TcpData,
                        src: addr_str(src),
                        dst: addr_str(dst),
                        sport: hdr.src_port,
                        dport: hdr.dst_port,
                        outbound_hint: Some(name),
                        tuple: Some(key),
                        payload: data.to_vec(),
                    }),
                );
            }
            if hdr.flags & FLAG_FIN != 0 {
                conn.state = TcpState::FinWait;
                return (
                    Vec::new(),
                    Some(StackEvent {
                        kind: StackEventKind::TcpFin,
                        src: addr_str(src),
                        dst: addr_str(dst),
                        sport: hdr.src_port,
                        dport: hdr.dst_port,
                        outbound_hint: Some(conn.outbound_name.clone()),
                        tuple: Some(key),
                        payload: Vec::new(),
                    }),
                );
            }
            if hdr.flags & FLAG_RST != 0 {
                conn.state = TcpState::Closed;
                return (
                    Vec::new(),
                    Some(StackEvent {
                        kind: StackEventKind::TcpRst,
                        src: addr_str(src),
                        dst: addr_str(dst),
                        sport: hdr.src_port,
                        dport: hdr.dst_port,
                        outbound_hint: None,
                        tuple: Some(key),
                        payload: Vec::new(),
                    }),
                );
            }
        }
        (Vec::new(), None)
    }

    fn handle_udp(
        &mut self,
        src: [u8; 4],
        dst: [u8; 4],
        seg: &[u8],
    ) -> (Vec<Vec<u8>>, Option<StackEvent>) {
        let Some((hdr, _)) = parse_udp(seg) else {
            return (Vec::new(), None);
        };
        (
            Vec::new(),
            Some(StackEvent {
                kind: StackEventKind::UdpDatagram,
                src: addr_str(src),
                dst: addr_str(dst),
                sport: hdr.src_port,
                dport: hdr.dst_port,
                outbound_hint: None,
                tuple: None,
                payload: Vec::new(),
            }),
        )
    }

    pub fn inject_tcp_data(&mut self, key: &FourTuple, data: &[u8]) -> Option<Vec<u8>> {
        let conn = self.conns.get_mut(key)?;
        if conn.state != TcpState::Established {
            return None;
        }
        let mut tcp = build_tcp(
            key.dport,
            key.sport,
            conn.server_seq,
            conn.client_seq,
            FLAG_ACK | FLAG_PSH,
            65535,
            data,
        );
        set_tcp_checksum(key.dst, key.src, &mut tcp);
        let pkt = build_ipv4(key.dst, key.src, 6, &tcp, self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        conn.server_seq = conn.server_seq.wrapping_add(data.len() as u32);
        conn.bytes_down = conn.bytes_down.saturating_add(data.len() as u64);
        self.packets_out = self.packets_out.saturating_add(1);
        Some(pkt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipv4::build_ipv4;
    use crate::tcp::build_tcp;

    #[test]
    fn syn_generates_synack() {
        let mut stack = NetStack::new();
        let tcp = build_tcp(40000, 443, 1000, 0, FLAG_SYN, 65535, &[]);
        let pkt = build_ipv4([10, 0, 0, 2], [1, 2, 3, 4], 6, &tcp, 1);
        let (replies, ev) = stack.handle_inbound(&pkt, "PROXY");
        assert_eq!(replies.len(), 1);
        let e = ev.unwrap();
        assert!(matches!(e.kind, StackEventKind::TcpSyn));
        assert!(e.tuple.is_some());
        assert_eq!(stack.conn_count(), 1);
    }
}
