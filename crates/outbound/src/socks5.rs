//! SOCKS5 client dialer.

use std::io::{Read, Write};

use netpilot_proxy::ProxyProfile;

use crate::addr::{encode_socks_addr, TargetAddr};
use crate::{connect_server, DialReport, DialRequest, OutboundError, OutboundStream};

pub fn dial_socks5(
    profile: &ProxyProfile,
    req: &DialRequest,
) -> Result<(OutboundStream, DialReport), OutboundError> {
    let started = std::time::Instant::now();
    let mut stream = connect_server(&profile.server, profile.port, req.timeout)?;

    // greeting: ver=5, nmethods=1, method=0 (no auth) or 2 (user/pass)
    let use_userpass = profile.username.is_some();
    if use_userpass {
        stream
            .write_all(&[0x05, 0x01, 0x02])
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    } else {
        stream
            .write_all(&[0x05, 0x01, 0x00])
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    }
    let mut resp = [0u8; 2];
    stream
        .read_exact(&mut resp)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    if resp[0] != 0x05 {
        return Err(OutboundError::Handshake("bad socks version".into()));
    }
    if use_userpass {
        if resp[1] != 0x02 {
            return Err(OutboundError::Handshake("server rejected user/pass".into()));
        }
        let user = profile.username.as_deref().unwrap_or("");
        let pass = profile.password.as_deref().unwrap_or("");
        if user.len() > 255 || pass.len() > 255 {
            return Err(OutboundError::Invalid("user/pass too long".into()));
        }
        let mut auth = Vec::with_capacity(3 + user.len() + pass.len());
        auth.push(0x01);
        auth.push(user.len() as u8);
        auth.extend_from_slice(user.as_bytes());
        auth.push(pass.len() as u8);
        auth.extend_from_slice(pass.as_bytes());
        stream
            .write_all(&auth)
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        let mut ar = [0u8; 2];
        stream
            .read_exact(&mut ar)
            .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        if ar[1] != 0x00 {
            return Err(OutboundError::Handshake("socks auth failed".into()));
        }
    } else if resp[1] != 0x00 {
        return Err(OutboundError::Handshake(format!(
            "unsupported method {}",
            resp[1]
        )));
    }

    // CONNECT request
    let mut req_buf = vec![0x05, 0x01, 0x00];
    let addr = TargetAddr::from_host_port(&req.target_host, req.target_port);
    req_buf.extend(encode_socks_addr(&addr)?);
    stream
        .write_all(&req_buf)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;

    // response: ver, rep, rsv, atyp, bind addr...
    let mut hdr = [0u8; 4];
    stream
        .read_exact(&mut hdr)
        .map_err(|e| OutboundError::Handshake(e.to_string()))?;
    if hdr[0] != 0x05 {
        return Err(OutboundError::Handshake("bad response version".into()));
    }
    if hdr[1] != 0x00 {
        return Err(OutboundError::Handshake(format!(
            "socks connect failed code={}",
            hdr[1]
        )));
    }
    consume_socks_addr(&mut stream, hdr[3])?;

    let peer = stream
        .peer_addr()
        .map(|x| x.to_string())
        .unwrap_or_default();
    Ok((
        OutboundStream::Plain(stream),
        DialReport {
            protocol: "socks5".into(),
            server: format!("{}:{}", profile.server, profile.port),
            peer,
            elapsed_ms: started.elapsed().as_millis(),
            via: profile.id.clone(),
        },
    ))
}

fn consume_socks_addr(stream: &mut impl Read, atyp: u8) -> Result<(), OutboundError> {
    match atyp {
        0x01 => {
            let mut b = [0u8; 4 + 2];
            stream
                .read_exact(&mut b)
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        }
        0x04 => {
            let mut b = [0u8; 16 + 2];
            stream
                .read_exact(&mut b)
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream
                .read_exact(&mut len)
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
            let mut b = vec![0u8; len[0] as usize + 2];
            stream
                .read_exact(&mut b)
                .map_err(|e| OutboundError::Handshake(e.to_string()))?;
        }
        _ => return Err(OutboundError::Handshake(format!("bad atyp {atyp}"))),
    }
    Ok(())
}
