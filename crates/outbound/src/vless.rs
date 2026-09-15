//! VLESS TCP client header (version 0) over TCP/TLS/WebSocket.

use std::io::Write;

use netpilot_proxy::{ProxyProfile, TransportKind};

use crate::addr::{encode_socks_addr, TargetAddr};
use crate::tls_stream::wrap_tls;
use crate::websocket::{connect_websocket, ws_send_binary, WsUpgrade};
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

    let is_ws = matches!(profile.transport, Some(TransportKind::Websocket));
    let use_tls = matches!(
        profile.transport,
        None | Some(TransportKind::Tls) | Some(TransportKind::Reality)
    ) || (is_ws && profile.sni.is_some())
        || profile.port == 443;

    let mut stream: OutboundStream = if is_ws {
        let host = profile
            .host
            .clone()
            .or_else(|| profile.sni.clone())
            .unwrap_or_else(|| profile.server.clone());
        let path = profile.path.clone().unwrap_or_else(|| "/".into());
        connect_websocket(
            &profile.server,
            profile.port,
            &WsUpgrade {
                host,
                path,
                sni: profile.sni.clone(),
                use_tls: use_tls || profile.sni.is_some(),
                timeout: req.timeout,
            },
        )?
    } else if use_tls {
        let tcp = connect_server(&profile.server, profile.port, req.timeout)?;
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
        OutboundStream::Plain(connect_server(&profile.server, profile.port, req.timeout)?)
    };

    let mut buf = Vec::with_capacity(64);
    buf.push(0x00);
    buf.extend_from_slice(&uuid_bytes);
    buf.push(0x00);
    buf.push(0x01);
    buf.extend_from_slice(&req.target_port.to_be_bytes());
    let addr = TargetAddr::from_host_port(&req.target_host, req.target_port);
    let mut socks = encode_socks_addr(&addr)?;
    if socks.len() < 3 {
        return Err(OutboundError::Invalid("addr encode too short".into()));
    }
    socks.truncate(socks.len() - 2);
    buf.extend_from_slice(&socks);

    if is_ws {
        ws_send_binary(&mut stream, &buf)?;
    } else {
        stream
            .write_all(&buf)
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    }

    Ok((
        stream,
        DialReport {
            protocol: if is_ws {
                "vless+ws".into()
            } else {
                "vless".into()
            },
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
