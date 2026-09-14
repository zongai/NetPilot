//! ShadowsocksR outbound (legacy compatibility subset).

use std::io::Write;

use netpilot_proxy::ProxyProfile;
use sha2::{Digest, Sha256};

use crate::addr::{encode_socks_addr, TargetAddr};
use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

/// Minimal SSR-like dial: TCP connect + obfuscated length prefix of socks address.
/// Full protocol chain (auth/protocol/obfs) is not bit-compatible with every SSR build.
pub fn dial_ssr(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let password = profile
        .password
        .as_deref()
        .ok_or_else(|| OutboundError::Invalid("ssr password required".into()))?;
    let mut stream = connect_server(&profile.server, profile.port, req.timeout)?;
    let addr = TargetAddr::from_host_port(&req.target_host, req.target_port);
    let body = encode_socks_addr(&addr)?;
    // Simple keyed scramble using SHA256(password) XOR (not production SSR)
    let key = Sha256::digest(password.as_bytes());
    let mut framed = Vec::with_capacity(2 + body.len());
    framed.extend_from_slice(&(body.len() as u16).to_be_bytes());
    for (i, b) in body.iter().enumerate() {
        framed.push(b ^ key[i % 32]);
    }
    stream
        .write_all(&framed)
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
