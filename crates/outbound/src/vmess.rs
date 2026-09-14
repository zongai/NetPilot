//! VMess outbound dialer.

use std::io::Write;

use netpilot_proxy::{ProxyProfile, TransportKind};
use netpilot_protocol_vmess::{build_vmess_request, parse_uuid, VmessConfig};

use crate::tls_stream::wrap_tls;
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

    let tcp = connect_server(&profile.server, profile.port, req.timeout)?;
    let use_tls = matches!(
        profile.transport,
        Some(TransportKind::Tls)
            | Some(TransportKind::Websocket)
            | Some(TransportKind::Reality)
    ) || profile.sni.is_some();

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

    let header = build_vmess_request(&cfg, &req.target_host, req.target_port)
        .map_err(OutboundError::Handshake)?;
    stream
        .write_all(&header)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    stream
        .flush()
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    Ok((
        stream,
        DialReport {
            protocol: "vmess".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer: format!("{}:{}", profile.server, profile.port),
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}
