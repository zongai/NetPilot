//! Trojan client: TLS + SHA224(password) hex + CRLF + SOCKS-like command.

use std::io::Write;

use netpilot_proxy::ProxyProfile;
use sha2::{Digest, Sha224};

use crate::addr::{encode_socks_addr, TargetAddr};
use crate::tls_stream::wrap_tls;
use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

pub fn dial_trojan(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let password = profile
        .password
        .as_deref()
        .ok_or_else(|| OutboundError::Invalid("trojan password required".into()))?;

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
    let alpn_refs: Vec<&str> = if alpn.is_empty() { vec![] } else { alpn };

    let mut tls = wrap_tls(tcp, sni, &alpn_refs, false)?;

    // hex(SHA224(password)) + \r\n
    let hash = Sha224::digest(password.as_bytes());
    let hex_hash = hex::encode(hash);
    let mut buf = Vec::with_capacity(128);
    buf.extend_from_slice(hex_hash.as_bytes());
    buf.extend_from_slice(b"\r\n");
    // CMD CONNECT = 0x01, RSV = 0x00 implied in socks addr layout after cmd
    buf.push(0x01); // CMD
    let addr = TargetAddr::from_host_port(&req.target_host, req.target_port);
    buf.extend(encode_socks_addr(&addr)?);
    buf.extend_from_slice(b"\r\n");

    tls.write_all(&buf)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    tls.flush()
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    Ok((
        OutboundStream::Tls(Box::new(tls)),
        DialReport {
            protocol: "trojan".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer: format!("{}:{}", profile.server, profile.port),
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}
