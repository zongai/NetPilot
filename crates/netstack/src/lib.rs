//! Userspace IPv4 / TCP / UDP stack for the TUN packet path.
//!
//! Handles SYN handshake, connection table, and segment build/parse so the
//! engine can map flows onto outbound dialers without a kernel netstack.

#![forbid(unsafe_code)]

mod conn;
mod ipv4;
mod stack;
mod tcp;
mod udp;

pub use conn::{ConnTable, FourTuple, TcpConn};
pub use ipv4::{addr_str, build_ipv4, parse_ipv4, Ipv4Header};
pub use stack::{NetStack, StackEvent, StackEventKind};
pub use tcp::{
    build_tcp, parse_tcp, set_tcp_checksum, TcpHeader, TcpState, FLAG_ACK, FLAG_FIN, FLAG_RST,
    FLAG_SYN,
};
pub use udp::{build_udp, parse_udp, UdpHeader};

pub const CRATE_NAME: &str = "netpilot-netstack";
