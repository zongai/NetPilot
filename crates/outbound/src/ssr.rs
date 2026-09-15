//! ShadowsocksR outbound using protocol crate stream cipher + auth header.

use std::io::Write;

use netpilot_protocol_shadowsocksr::{build_tcp_request, SsrConfig, SsrMethod, SsrProtocol};
use netpilot_proxy::ProxyProfile;

use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

pub fn dial_ssr(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let password = profile
        .password
        .as_deref()
        .ok_or_else(|| OutboundError::Invalid("ssr password required".into()))?;

    let mut cfg = SsrConfig::new(&profile.server, profile.port, password);
    if let Some(ref m) = profile.cipher {
        cfg.method = SsrMethod::parse(m);
    }
    // protocol / obfs from tags or flow field when present
    if let Some(ref flow) = profile.flow {
        for part in flow.split(';') {
            let p = part.trim();
            if let Some(v) = p.strip_prefix("protocol=") {
                cfg.protocol = SsrProtocol::parse(v);
            } else if let Some(v) = p.strip_prefix("obfs=") {
                cfg.obfs = v.to_string();
            } else if let Some(v) = p.strip_prefix("protocol_param=") {
                cfg.protocol_param = v.to_string();
            } else if let Some(v) = p.strip_prefix("obfs_param=") {
                cfg.obfs_param = v.to_string();
            }
        }
    }

    let mut stream = connect_server(&profile.server, profile.port, req.timeout)?;
    let (pkt, _cipher) = build_tcp_request(&cfg, &req.target_host, req.target_port, &[])
        .map_err(OutboundError::Handshake)?;
    stream
        .write_all(&pkt)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    stream
        .flush()
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    Ok((
        OutboundStream::Plain(stream),
        DialReport {
            protocol: "shadowsocksr".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer: format!("{}:{}", profile.server, profile.port),
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}
