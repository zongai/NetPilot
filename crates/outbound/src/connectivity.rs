//! Real connectivity probe: TCP → proxy/protocol → TLS → HTTPS GET.
//! Not a substitute for IPC `ping`.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use netpilot_proxy::ProxyProfile;

use crate::{
    dial_direct, dial_via_profile, wrap_tls, DialReport, DialRequest, OutboundError, OutboundStream,
};

#[derive(Debug, Clone)]
pub struct ConnectivityStage {
    pub name: String,
    pub ok: bool,
    pub detail: String,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone)]
pub struct ConnectivityReport {
    pub ok: bool,
    pub target: String,
    pub via: String,
    pub stages: Vec<ConnectivityStage>,
    pub http_status: Option<u16>,
    pub peer: Option<String>,
    pub elapsed_ms: u128,
    pub error: Option<String>,
}

/// Probe HTTPS (or plain HTTP) through an optional proxy profile.
///
/// Stages recorded:
/// 1. `dial` — TCP to proxy/target + protocol handshake
/// 2. `tls` — TLS client handshake when port is 443 (or `force_tls`)
/// 3. `http` — `GET /` and parse status line
pub fn probe_http_connectivity(
    profile: Option<&ProxyProfile>,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
    force_tls: bool,
) -> ConnectivityReport {
    let started = Instant::now();
    let mut stages = Vec::new();
    let target = format!("{target_host}:{target_port}");
    let via = profile
        .map(|p| format!("{}://{}:{}", p.protocol.as_str(), p.server, p.port))
        .unwrap_or_else(|| "direct".into());

    let mut req = DialRequest::new(target_host, target_port);
    req.timeout = timeout;

    let dial_start = Instant::now();
    let dial_result = match profile {
        Some(p) => dial_via_profile(p, &req),
        None => dial_direct(&req),
    };

    let (stream, dial_report) = match dial_result {
        Ok(v) => {
            stages.push(ConnectivityStage {
                name: "dial".into(),
                ok: true,
                detail: format!(
                    "protocol={} peer={} via={}",
                    v.1.protocol, v.1.peer, v.1.via
                ),
                elapsed_ms: dial_start.elapsed().as_millis(),
            });
            v
        }
        Err(e) => {
            stages.push(ConnectivityStage {
                name: "dial".into(),
                ok: false,
                detail: e.to_string(),
                elapsed_ms: dial_start.elapsed().as_millis(),
            });
            return ConnectivityReport {
                ok: false,
                target,
                via,
                stages,
                http_status: None,
                peer: None,
                elapsed_ms: started.elapsed().as_millis(),
                error: Some(e.to_string()),
            };
        }
    };
    let peer = Some(dial_report.peer.clone());

    let need_tls = force_tls || target_port == 443;
    let mut stream = stream;
    if need_tls {
        let tls_start = Instant::now();
        match ensure_tls(stream, target_host) {
            Ok(s) => {
                stages.push(ConnectivityStage {
                    name: "tls".into(),
                    ok: true,
                    detail: format!("SNI={target_host}"),
                    elapsed_ms: tls_start.elapsed().as_millis(),
                });
                stream = s;
            }
            Err(e) => {
                stages.push(ConnectivityStage {
                    name: "tls".into(),
                    ok: false,
                    detail: e.to_string(),
                    elapsed_ms: tls_start.elapsed().as_millis(),
                });
                return ConnectivityReport {
                    ok: false,
                    target,
                    via,
                    stages,
                    http_status: None,
                    peer,
                    elapsed_ms: started.elapsed().as_millis(),
                    error: Some(e.to_string()),
                };
            }
        }
    } else {
        stages.push(ConnectivityStage {
            name: "tls".into(),
            ok: true,
            detail: "skipped (non-TLS port)".into(),
            elapsed_ms: 0,
        });
    }

    let http_start = Instant::now();
    match http_get_status(&mut stream, target_host) {
        Ok(status) => {
            let ok = (200..400).contains(&status);
            stages.push(ConnectivityStage {
                name: "http".into(),
                ok,
                detail: format!("status={status}"),
                elapsed_ms: http_start.elapsed().as_millis(),
            });
            ConnectivityReport {
                ok,
                target,
                via,
                stages,
                http_status: Some(status),
                peer,
                elapsed_ms: started.elapsed().as_millis(),
                error: if ok {
                    None
                } else {
                    Some(format!("unexpected HTTP status {status}"))
                },
            }
        }
        Err(e) => {
            stages.push(ConnectivityStage {
                name: "http".into(),
                ok: false,
                detail: e.clone(),
                elapsed_ms: http_start.elapsed().as_millis(),
            });
            ConnectivityReport {
                ok: false,
                target,
                via,
                stages,
                http_status: None,
                peer,
                elapsed_ms: started.elapsed().as_millis(),
                error: Some(e),
            }
        }
    }
}

fn ensure_tls(stream: OutboundStream, server_name: &str) -> Result<OutboundStream, OutboundError> {
    match stream {
        OutboundStream::Tls(t) => Ok(OutboundStream::Tls(t)),
        OutboundStream::Plain(tcp) => {
            let tls = wrap_tls(tcp, server_name, &["http/1.1"], false)?;
            Ok(OutboundStream::Tls(Box::new(tls)))
        }
    }
}

fn http_get_status(stream: &mut OutboundStream, host: &str) -> Result<u16, String> {
    let req = format!(
        "GET / HTTP/1.1\r\nHost: {host}\r\nUser-Agent: NetPilot-Connectivity/1.0\r\nConnection: close\r\nAccept: */*\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().map_err(|e| format!("http flush: {e}"))?;

    let mut buf = [0u8; 2048];
    let mut got = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() > deadline {
            return Err("http read timeout".into());
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                got.extend_from_slice(&buf[..n]);
                if got.windows(4).any(|w| w == b"\r\n\r\n") || got.len() > 4096 {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                if got.is_empty() {
                    return Err("http read timeout".into());
                }
                break;
            }
            Err(e) => return Err(format!("http read: {e}")),
        }
    }
    let text = String::from_utf8_lossy(&got);
    let line = text.lines().next().unwrap_or("");
    // HTTP/1.1 200 OK
    let mut parts = line.split_whitespace();
    let ver = parts.next().unwrap_or("");
    if !ver.starts_with("HTTP/") {
        return Err(format!("bad status line: {line:?}"));
    }
    let code = parts
        .next()
        .ok_or_else(|| format!("no status code in {line:?}"))?;
    code.parse::<u16>()
        .map_err(|_| format!("invalid status code in {line:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_structure_on_dial_fail() {
        // Unlikely-to-exist proxy — must fail at dial, not pretend ok.
        let profile = ProxyProfile {
            id: "x".into(),
            name: "x".into(),
            protocol: netpilot_proxy::ProtocolKind::Socks5,
            transport: None,
            server: "127.0.0.1".into(),
            port: 1,
            password: None,
            uuid: None,
            username: None,
            sni: None,
            alpn: None,
            path: None,
            host: None,
            flow: None,
            network: None,
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        };
        let r = probe_http_connectivity(
            Some(&profile),
            "example.com",
            443,
            Duration::from_millis(300),
            true,
        );
        assert!(!r.ok);
        assert_eq!(r.stages[0].name, "dial");
        assert!(!r.stages[0].ok);
    }

    #[test]
    fn direct_https_example_com() {
        let r = probe_http_connectivity(None, "example.com", 443, Duration::from_secs(15), true);
        assert!(r.ok, "expected HTTPS path ok, got {r:?}");
        assert_eq!(r.http_status.unwrap_or(0) / 100, 2);
        assert!(r.stages.iter().any(|s| s.name == "dial" && s.ok));
        assert!(r.stages.iter().any(|s| s.name == "tls" && s.ok));
        assert!(r.stages.iter().any(|s| s.name == "http" && s.ok));
    }
}
