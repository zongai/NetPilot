//! HTTP CONNECT proxy dialer.

use std::io::{Read, Write};

use netpilot_proxy::ProxyProfile;

use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

pub fn dial_http_connect(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let mut stream = connect_server(&profile.server, profile.port, req.timeout)?;

    let mut request = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n",
        req.target_host, req.target_port, req.target_host, req.target_port
    );
    if let (Some(user), pass) = (
        profile.username.as_deref(),
        profile.password.as_deref().unwrap_or(""),
    ) {
        let token = base64_encode(&format!("{user}:{pass}"));
        request.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    // Read response headers until \r\n\r\n
    let mut buf = Vec::with_capacity(256);
    let mut tmp = [0u8; 1];
    loop {
        stream
            .read_exact(&mut tmp)
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        buf.push(tmp[0]);
        if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if buf.len() > 8192 {
            return Err(OutboundError::Handshake("response too large".into()));
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let status_line = text.lines().next().unwrap_or("");
    if !status_line.contains("200") {
        return Err(OutboundError::Handshake(format!(
            "CONNECT failed: {status_line}"
        )));
    }

    let peer = stream
        .peer_addr()
        .map(|x| x.to_string())
        .unwrap_or_default();
    Ok((
        OutboundStream::Plain(stream),
        DialReport {
            protocol: "http".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer,
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}

fn base64_encode(s: &str) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((triple >> 18) & 63) as usize] as char);
        out.push(T[((triple >> 12) & 63) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(T[((triple >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(T[(triple & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}
