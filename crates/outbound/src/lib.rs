//! Outbound dialers: Direct / SOCKS5 / HTTP CONNECT / Trojan / VLESS / Shadowsocks.
//!
//! Provides blocking stream-oriented dial that returns a connected `TcpStream`
//! (or TLS-wrapped stream via [`OutboundStream`]). Used by the traffic engine
//! after rule decisions.

#![forbid(unsafe_code)]

mod addr;
mod http_connect;
mod shadowsocks;
mod socks5;
mod tls_stream;
mod trojan;
mod vless;
mod vmess;
mod websocket;
mod ssr;

pub use addr::{encode_socks_addr, TargetAddr};
pub use http_connect::dial_http_connect;
pub use shadowsocks::dial_shadowsocks;
pub use socks5::dial_socks5;
pub use tls_stream::{wrap_tls, TlsStream};
pub use trojan::dial_trojan;
pub use vless::dial_vless;
pub use vmess::dial_vmess;
pub use websocket::{connect_websocket, ws_send_binary, WsUpgrade};
pub use ssr::dial_ssr;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use netpilot_proxy::{ProtocolKind, ProxyProfile};

pub const CRATE_NAME: &str = "netpilot-outbound";

#[derive(Debug)]
pub enum OutboundError {
    Invalid(String),
    Dial(String),
    Handshake(String),
    Tls(String),
    Rejected,
    Unsupported(String),
}

impl std::fmt::Display for OutboundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(m) => write!(f, "Invalid({m})"),
            Self::Dial(m) => write!(f, "Dial({m})"),
            Self::Handshake(m) => write!(f, "Handshake({m})"),
            Self::Tls(m) => write!(f, "Tls({m})"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Unsupported(m) => write!(f, "Unsupported({m})"),
        }
    }
}

impl std::error::Error for OutboundError {}

/// Connected outbound stream (plain TCP or TLS).
pub enum OutboundStream {
    Plain(TcpStream),
    Tls(Box<TlsStream>),
}

impl std::fmt::Debug for OutboundStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(_) => write!(f, "OutboundStream::Plain(..)"),
            Self::Tls(_) => write!(f, "OutboundStream::Tls(..)"),
        }
    }
}

impl OutboundStream {
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.set_read_timeout(timeout),
            Self::Tls(s) => s.set_read_timeout(timeout),
        }
    }

    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.set_write_timeout(timeout),
            Self::Tls(s) => s.set_write_timeout(timeout),
        }
    }
}

impl Read for OutboundStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for OutboundStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DialRequest {
    /// Final destination the client wants to reach.
    pub target_host: String,
    pub target_port: u16,
    pub timeout: Duration,
}

impl DialRequest {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            target_host: host.into(),
            target_port: port,
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DialReport {
    pub protocol: String,
    pub server: String,
    pub peer: String,
    pub elapsed_ms: u128,
    pub via: String,
}

/// Direct TCP to target (no proxy).
pub fn dial_direct(req: &DialRequest) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let addr = format!("{}:{}", req.target_host.trim(), req.target_port);
    let addrs = addr.to_socket_addrs_safe().map_err(OutboundError::Dial)?;
    let deadline = started + req.timeout;
    let mut last = OutboundError::Dial("no address".into());
    for a in addrs {
        let remain = deadline.saturating_duration_since(std::time::Instant::now());
        if remain.is_zero() {
            return Err(OutboundError::Dial("timeout".into()));
        }
        match TcpStream::connect_timeout(&a, remain) {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|_| a.to_string());
                let _ = stream.set_nodelay(true);
                return Ok((
                    OutboundStream::Plain(stream),
                    DialReport {
                        protocol: "direct".into(),
                        server: format!("{}:{}", req.target_host, req.target_port),
                        peer,
                        elapsed_ms: started.elapsed().as_millis(),
                        via: "direct".into(),
                    },
                ));
            }
            Err(e) => last = OutboundError::Dial(e.to_string()),
        }
    }
    Err(last)
}

trait ToSocketAddrsExt {
    fn to_socket_addrs_safe(&self) -> Result<Vec<std::net::SocketAddr>, String>;
}

impl ToSocketAddrsExt for String {
    fn to_socket_addrs_safe(&self) -> Result<Vec<std::net::SocketAddr>, String> {
        use std::net::ToSocketAddrs;
        self.to_socket_addrs()
            .map(|i| i.collect())
            .map_err(|e| e.to_string())
    }
}

impl ToSocketAddrsExt for str {
    fn to_socket_addrs_safe(&self) -> Result<Vec<std::net::SocketAddr>, String> {
        use std::net::ToSocketAddrs;
        self.to_socket_addrs()
            .map(|i| i.collect())
            .map_err(|e| e.to_string())
    }
}

/// Dial through a proxy profile toward `req` target.
pub fn dial_via_profile(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    match profile.protocol {
        ProtocolKind::Socks5 => dial_socks5(profile, req),
        ProtocolKind::Http => dial_http_connect(profile, req),
        ProtocolKind::Trojan => dial_trojan(profile, req),
        ProtocolKind::Vless => dial_vless(profile, req),
        ProtocolKind::Shadowsocks | ProtocolKind::Shadowsocks2022 => dial_shadowsocks(profile, req),
        ProtocolKind::Vmess => dial_vmess(profile, req),
        ProtocolKind::ShadowsocksR => dial_ssr(profile, req),
    }
}

/// Resolve outbound name from rules and dial.
///
/// - `DIRECT` → direct TCP
/// - `REJECT` → error
/// - other → look up profile by id or name
pub fn dial_outbound(
    outbound: &str,
    profiles: &[ProxyProfile],
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let name = outbound.trim();
    if name.eq_ignore_ascii_case("DIRECT") {
        return dial_direct(req);
    }
    if name.eq_ignore_ascii_case("REJECT") || name.eq_ignore_ascii_case("BLOCK") {
        return Err(OutboundError::Rejected);
    }
    let profile = profiles
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(name) || p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| OutboundError::Invalid(format!("unknown outbound '{name}'")))?;
    dial_via_profile(profile, req)
}

/// TCP connect helper to proxy server itself.
pub(crate) fn connect_server(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<TcpStream, OutboundError> {
    let addr = format!("{}:{}", host.trim(), port);
    let addrs = addr.to_socket_addrs_safe().map_err(OutboundError::Dial)?;
    let started = std::time::Instant::now();
    let deadline = started + timeout;
    let mut last = OutboundError::Dial("no address".into());
    for a in addrs {
        let remain = deadline.saturating_duration_since(std::time::Instant::now());
        if remain.is_zero() {
            return Err(OutboundError::Dial("timeout".into()));
        }
        match TcpStream::connect_timeout(&a, remain) {
            Ok(s) => {
                let _ = s.set_nodelay(true);
                return Ok(s);
            }
            Err(e) => last = OutboundError::Dial(e.to_string()),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_loopback() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let _ = listener.accept();
        });
        let (mut stream, report) = dial_direct(&DialRequest::new("127.0.0.1", port)).unwrap();
        assert_eq!(report.protocol, "direct");
        let _ = stream.write_all(b"hi");
        let _ = handle.join();
    }

    #[test]
    fn reject_outbound() {
        let err = dial_outbound("REJECT", &[], &DialRequest::new("example.com", 80)).unwrap_err();
        assert!(matches!(err, OutboundError::Rejected));
    }
}
