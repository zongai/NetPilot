//! VLESS TCP client header (version 0) over TLS when transport is TLS/REALITY.

use std::io::Write;

use netpilot_proxy::{ProxyProfile, TransportKind};

use crate::addr::{encode_socks_addr, TargetAddr};
use crate::tls_stream::wrap_tls;
use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

pub fn dial_vless(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let uuid = profile
        .uuid
        .as_deref()
        .ok_or_else(|| OutboundError::Invalid("vless uuid required".into()))?;
    let uuid_bytes = parse_uuid(uuid)?;

    let tcp = connect_server(&profile.server, profile.port, req.timeout)?;

    let use_tls = matches!(
        profile.transport,
        None | Some(TransportKind::Tls)
            | Some(TransportKind::Reality)
            | Some(TransportKind::Websocket)
    );

    let mut stream: OutboundStream = if use_tls {
        let sni = profile
            .sni
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(profile.server.as_str());
        let alpn: Vec<&str> = profile
            .alpn
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let tls = wrap_tls(tcp, sni, &alpn, false)?;
        OutboundStream::Tls(Box::new(tls))
    } else {
        OutboundStream::Plain(tcp)
    };

    // VLESS request header:
    // version(1) + uuid(16) + addon_len(1) + addon + command(1) + port(2) + addr
    let mut buf = Vec::with_capacity(64);
    buf.push(0x00); // version
    buf.extend_from_slice(&uuid_bytes);
    buf.push(0x00); // addon length 0
    buf.push(0x01); // command TCP
    buf.extend_from_slice(&req.target_port.to_be_bytes());
    // address: reuse socks atyp encoding without port (VLESS puts port before addr)
    let addr = TargetAddr::from_host_port(&req.target_host, req.target_port);
    let mut socks = encode_socks_addr(&addr)?;
    // socks is atyp + addr + port; strip trailing port (2 bytes)
    if socks.len() < 3 {
        return Err(OutboundError::Invalid("addr encode too short".into()));
    }
    socks.truncate(socks.len() - 2);
    buf.extend_from_slice(&socks);

    stream
        .write_all(&buf)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    stream
        .flush()
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    Ok((
        stream,
        DialReport {
            protocol: "vless".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer: format!("{}:{}", profile.server, profile.port),
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}

fn parse_uuid(s: &str) -> Result<[u8; 16], OutboundError> {
    let s = s.trim();
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return Err(OutboundError::Invalid("uuid must be 32 hex chars".into()));
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| OutboundError::Invalid("bad uuid hex".into()))?;
    }
    Ok(out)
}
