//! URI list parser: ss/ssr/vmess/vless/trojan (NP-130).

use std::collections::HashMap;

use super::ParsedNode;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriParseError {
    Unsupported,
    Malformed,
}

impl std::fmt::Display for UriParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Unsupported"),
            Self::Malformed => write!(f, "Malformed"),
        }
    }
}

impl std::error::Error for UriParseError {}

pub fn parse_uri_list(text: &str) -> Vec<Result<ParsedNode, UriParseError>> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(parse_one_uri)
        .collect()
}

fn parse_one_uri(line: &str) -> Result<ParsedNode, UriParseError> {
    let (scheme, rest) = line.split_once("://").ok_or(UriParseError::Malformed)?;
    match scheme.to_ascii_lowercase().as_str() {
        "ss" => parse_ss(rest),
        "ssr" => parse_ssr(rest),
        "vmess" => parse_vmess(rest),
        "vless" => parse_vless_or_trojan(rest, "vless"),
        "trojan" => parse_vless_or_trojan(rest, "trojan"),
        _ => Err(UriParseError::Unsupported),
    }
}

fn parse_ss(rest: &str) -> Result<ParsedNode, UriParseError> {
    // ss://base64(method:pass@host:port)#name  OR ss://method:pass@host:port
    let (main, name) = rest.split_once('#').unwrap_or((rest, "ss"));
    let name = url_decode(name);
    let decoded = if main.contains('@') {
        main.to_string()
    } else {
        String::from_utf8(crate::decoder::decode_base64_bytes(main).unwrap_or_default())
            .unwrap_or_default()
    };
    let (userinfo, hostport) = decoded.split_once('@').ok_or(UriParseError::Malformed)?;
    let (method, password) = userinfo.split_once(':').unwrap_or((userinfo, ""));
    let (server, port) = split_host_port(hostport)?;
    let mut params = HashMap::new();
    params.insert("method".into(), method.into());
    Ok(ParsedNode {
        name,
        protocol: "shadowsocks".into(),
        server,
        port,
        password: Some(password.into()),
        uuid: None,
        params,
        source_format: "uri-ss",
    })
}

fn parse_ssr(rest: &str) -> Result<ParsedNode, UriParseError> {
    let raw = String::from_utf8(crate::decoder::decode_base64_bytes(rest).unwrap_or_default())
        .unwrap_or_default();
    // host:port:protocol:method:obfs:base64pass
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() < 6 {
        return Err(UriParseError::Malformed);
    }
    let server = parts[0].to_string();
    let port: u16 = parts[1].parse().map_err(|_| UriParseError::Malformed)?;
    let mut params = HashMap::new();
    params.insert("protocol".into(), parts[2].into());
    params.insert("method".into(), parts[3].into());
    params.insert("obfs".into(), parts[4].into());
    Ok(ParsedNode {
        name: "ssr".into(),
        protocol: "shadowsocksr".into(),
        server,
        port,
        password: Some(parts[5].into()),
        uuid: None,
        params,
        source_format: "uri-ssr",
    })
}

fn parse_vmess(rest: &str) -> Result<ParsedNode, UriParseError> {
    // vmess://base64(json) — minimal: extract add/port/id/ps from json-ish text
    let json = String::from_utf8(crate::decoder::decode_base64_bytes(rest).unwrap_or_default())
        .unwrap_or_default();
    let server = json_field(&json, "add").unwrap_or_default();
    let port: u16 = json_field(&json, "port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    if server.is_empty() || port == 0 {
        return Err(UriParseError::Malformed);
    }
    let mut params = HashMap::new();
    if let Some(net) = json_field(&json, "net") {
        params.insert("network".into(), net);
    }
    if let Some(tls) = json_field(&json, "tls") {
        params.insert("tls".into(), tls);
    }
    Ok(ParsedNode {
        name: json_field(&json, "ps").unwrap_or_else(|| "vmess".into()),
        protocol: "vmess".into(),
        server,
        port,
        password: None,
        uuid: json_field(&json, "id"),
        params,
        source_format: "uri-vmess",
    })
}

fn parse_vless_or_trojan(rest: &str, protocol: &str) -> Result<ParsedNode, UriParseError> {
    // vless://uuid@host:port?params#name
    let (main, name) = rest.split_once('#').unwrap_or((rest, protocol));
    let name = url_decode(name);
    let (userinfo, host_q) = main.split_once('@').ok_or(UriParseError::Malformed)?;
    let (hostport, query) = host_q.split_once('?').unwrap_or((host_q, ""));
    let (server, port) = split_host_port(hostport)?;
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            params.insert(k.to_string(), url_decode(v));
        }
    }
    let (password, uuid) = if protocol == "trojan" {
        (Some(userinfo.to_string()), None)
    } else {
        (None, Some(userinfo.to_string()))
    };
    Ok(ParsedNode {
        name,
        protocol: protocol.into(),
        server,
        port,
        password,
        uuid,
        params,
        source_format: if protocol == "trojan" {
            "uri-trojan"
        } else {
            "uri-vless"
        },
    })
}

fn split_host_port(s: &str) -> Result<(String, u16), UriParseError> {
    let (h, p) = s.rsplit_once(':').ok_or(UriParseError::Malformed)?;
    let port: u16 = p.parse().map_err(|_| UriParseError::Malformed)?;
    Ok((h.trim_matches('[').trim_matches(']').to_string(), port))
}

fn json_field(json: &str, key: &str) -> Option<String> {
    // minimal "key":"value" extractor
    let patterns = [
        format!("\"{key}\":\""),
        format!("\"{key}\": \""),
        format!("\"{key}\":"),
    ];
    for pat in &patterns {
        if let Some(i) = json.find(pat) {
            let rest = &json[i + pat.len()..];
            if pat.ends_with(':') && !pat.ends_with("\"") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('"') {
                    if let Some(end) = rest.find('"') {
                        return Some(rest[..end].to_string());
                    }
                } else {
                    let end = rest
                        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
                        .unwrap_or(rest.len());
                    return Some(rest[..end].trim_matches('"').to_string());
                }
            } else if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (from_hex(b[i + 1]), from_hex(b[i + 2])) {
                out.push((h << 4 | l) as char);
                i += 3;
                continue;
            }
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

fn from_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trojan() {
        let nodes = parse_uri_list("trojan://pass@example.com:443?security=tls#node1\n");
        let n = nodes[0].as_ref().unwrap();
        assert_eq!(n.protocol, "trojan");
        assert_eq!(n.server, "example.com");
        assert_eq!(n.port, 443);
        assert_eq!(n.name, "node1");
    }
}
