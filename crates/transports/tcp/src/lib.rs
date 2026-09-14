//! TCP transport surface + outbound dial helper.

#![forbid(unsafe_code)]

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub use netpilot_protocol_common::TransportId;

pub const CRATE_NAME: &str = "netpilot-transport-tcp";

pub fn transport_id() -> TransportId {
    TransportId::Tcp
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialError {
    InvalidAddr(String),
    Timeout,
    Connect(String),
}

impl std::fmt::Display for DialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAddr(m) => write!(f, "InvalidAddr({m})"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Connect(m) => write!(f, "Connect({m})"),
        }
    }
}

impl std::error::Error for DialError {}

#[derive(Debug, Clone)]
pub struct TcpDialRequest {
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
}

impl TcpDialRequest {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[derive(Debug, Clone)]
pub struct TcpDialResult {
    pub peer: String,
    pub local: String,
    pub elapsed_ms: u128,
}

/// Blocking TCP connect probe (used by Core `outbound.tcp_probe`).
pub fn dial_tcp(req: &TcpDialRequest) -> Result<TcpDialResult, DialError> {
    if req.host.trim().is_empty() {
        return Err(DialError::InvalidAddr("empty host".into()));
    }
    if req.port == 0 {
        return Err(DialError::InvalidAddr("port 0".into()));
    }
    let addr = format!("{}:{}", req.host.trim(), req.port);
    let started = std::time::Instant::now();
    let mut addrs = addr
        .to_socket_addrs()
        .map_err(|e| DialError::InvalidAddr(e.to_string()))?
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err(DialError::InvalidAddr("no resolved addresses".into()));
    }
    // Prefer first resolved address; try in order until timeout budget exhausted.
    let deadline = started + req.timeout;
    let mut last_err = DialError::Connect("no attempt".into());
    for a in addrs.drain(..) {
        let remain = deadline.saturating_duration_since(std::time::Instant::now());
        if remain.is_zero() {
            return Err(DialError::Timeout);
        }
        match TcpStream::connect_timeout(&a, remain) {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|_| a.to_string());
                let local = stream
                    .local_addr()
                    .map(|x| x.to_string())
                    .unwrap_or_default();
                // Drop stream — probe only.
                drop(stream);
                return Ok(TcpDialResult {
                    peer,
                    local,
                    elapsed_ms: started.elapsed().as_millis(),
                });
            }
            Err(e) => {
                last_err = if e.kind() == std::io::ErrorKind::TimedOut {
                    DialError::Timeout
                } else {
                    DialError::Connect(e.to_string())
                };
            }
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id() {
        assert_eq!(transport_id(), TransportId::Tcp);
    }

    #[test]
    fn invalid_host() {
        let err = dial_tcp(&TcpDialRequest::new("", 80)).unwrap_err();
        assert!(matches!(err, DialError::InvalidAddr(_)));
    }

    #[test]
    fn loopback_probe() {
        // Bind ephemeral listener and dial it.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let _ = listener.accept();
        });
        let res = dial_tcp(
            &TcpDialRequest::new("127.0.0.1", port).with_timeout(Duration::from_secs(2)),
        )
        .unwrap();
        assert!(res.elapsed_ms < 2000);
        let _ = handle.join();
    }
}
