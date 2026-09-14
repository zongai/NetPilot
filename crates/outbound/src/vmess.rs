//! VMess outbound dialer (TCP/TLS/WebSocket).

use std::io::Write;

use netpilot_proxy::{ProxyProfile, TransportKind};
use netpilot_protocol_vmess::{build_vmess_request, parse_uuid, VmessConfig};

use crate::tls_stream::wrap_tls;
use crate::websocket::{connect_websocket, ws_send_binary, WsUpgrade};
use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

pub fn dial_vmess(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let uuid_str = profile
        .uuid
        .as_deref()
        .ok_or_else(|| OutboundError::Invalid("vmess uuid required".into()))?;
    let uuid = parse_uuid(uuid_str).map_err(OutboundError::Invalid)?;
    let cfg = VmessConfig::new(&profile.server, profile.port, uuid);

    let is_ws = matches!(profile.transport, Some(TransportKind::Websocket));
    let use_tls = matches!(
        profile.transport,
        Some(TransportKind::Tls) | Some(TransportKind::Reality)
    ) || profile.sni.is_some()
        || (is_ws && profile.port == 443);

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
                use_tls: use_tls || profile.sni.is_some() || profile.port == 443,
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
        OutboundStream::Tls(Box::new(wrap_tls(tcp, sni, &alpn, false)?))
    } else {
        OutboundStream::Plain(connect_server(
            &profile.server,
            profile.port,
            req.timeout,
        )?)
    };

    let header = build_vmess_request(&cfg, &req.target_host, req.target_port)
        .map_err(OutboundError::Handshake)?;
    if is_ws {
        ws_send_binary(&mut stream, &header)?;
    } else {
        stream
            .write_all(&header)
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        stream
            .flush()
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    }

    Ok((
        stream,
        DialReport {
            protocol: if is_ws {
                "vmess+ws".into()
            } else {
                "vmess".into()
            },
            server: format!("{}:{}", profile.server, profile.port),
            peer: format!("{}:{}", profile.server, profile.port),
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}
