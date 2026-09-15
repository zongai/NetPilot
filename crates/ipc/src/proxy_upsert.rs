//! `proxy.upsert` request schema (NP-INTEGRATION-002).
//!
//! Canonical payload:
//! ```json
//! {
//!   "id": "node-1",
//!   "name": "Demo SOCKS5",
//!   "server": "127.0.0.1",
//!   "port": 1080,
//!   "protocol": "socks5"
//! }
//! ```
//! Optional: `password`, `uuid`, `username`, `sni`, `alpn`.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyUpsertRequest {
    pub id: String,
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub password: Option<String>,
    pub uuid: Option<String>,
    pub username: Option<String>,
    pub sni: Option<String>,
    pub alpn: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyUpsertParseError {
    MissingPayload,
    EmptyPayload,
    MalformedPayload,
    MissingId,
    MissingServer,
    MissingPort,
    InvalidPort,
    UnknownProtocol,
}

impl ProxyUpsertParseError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingPayload => "missing payload",
            Self::EmptyPayload => "empty payload",
            Self::MalformedPayload => "malformed payload",
            Self::MissingId => "payload.id required",
            Self::MissingServer => "payload.server required",
            Self::MissingPort => "payload.port required",
            Self::InvalidPort => "payload.port invalid",
            Self::UnknownProtocol => "unknown protocol",
        }
    }
}

fn opt_string(v: Option<&Value>) -> Result<Option<String>, ProxyUpsertParseError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let t = s.trim();
            Ok(if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            })
        }
        Some(_) => Err(ProxyUpsertParseError::MalformedPayload),
    }
}

/// Known protocol identifiers accepted by Core (must match `ProtocolKind::parse`).
fn is_known_protocol(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "http"
            | "socks5"
            | "shadowsocks"
            | "ss"
            | "shadowsocks2022"
            | "shadowsocksr"
            | "ssr"
            | "vmess"
            | "vless"
            | "trojan"
    )
}

pub fn parse_proxy_upsert_payload(
    payload: Option<&Value>,
) -> Result<ProxyUpsertRequest, ProxyUpsertParseError> {
    let Some(payload) = payload else {
        return Err(ProxyUpsertParseError::MissingPayload);
    };
    if !payload.is_object() {
        return Err(ProxyUpsertParseError::MalformedPayload);
    }
    let obj = payload.as_object().unwrap();
    if obj.is_empty() {
        return Err(ProxyUpsertParseError::EmptyPayload);
    }

    let id = match obj.get("id") {
        None | Some(Value::Null) => return Err(ProxyUpsertParseError::MissingId),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                return Err(ProxyUpsertParseError::MissingId);
            }
            t.to_string()
        }
        Some(_) => return Err(ProxyUpsertParseError::MalformedPayload),
    };

    let name = match obj.get("name") {
        None | Some(Value::Null) => id.clone(),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                id.clone()
            } else {
                t.to_string()
            }
        }
        Some(_) => return Err(ProxyUpsertParseError::MalformedPayload),
    };

    let server = match obj.get("server") {
        None | Some(Value::Null) => return Err(ProxyUpsertParseError::MissingServer),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                return Err(ProxyUpsertParseError::MissingServer);
            }
            t.to_string()
        }
        Some(_) => return Err(ProxyUpsertParseError::MalformedPayload),
    };

    let port = match obj.get("port") {
        None | Some(Value::Null) => return Err(ProxyUpsertParseError::MissingPort),
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(ProxyUpsertParseError::InvalidPort);
            };
            if u == 0 || u > u16::MAX as u64 {
                return Err(ProxyUpsertParseError::InvalidPort);
            }
            u as u16
        }
        Some(_) => return Err(ProxyUpsertParseError::MalformedPayload),
    };

    let protocol = match obj.get("protocol") {
        None | Some(Value::Null) => "socks5".to_string(),
        Some(Value::String(s)) => {
            let t = s.trim().to_ascii_lowercase();
            if t.is_empty() {
                "socks5".to_string()
            } else if !is_known_protocol(&t) {
                return Err(ProxyUpsertParseError::UnknownProtocol);
            } else {
                t
            }
        }
        Some(_) => return Err(ProxyUpsertParseError::MalformedPayload),
    };

    Ok(ProxyUpsertRequest {
        id,
        name,
        server,
        port,
        protocol,
        password: opt_string(obj.get("password"))?,
        uuid: opt_string(obj.get("uuid"))?,
        username: opt_string(obj.get("username"))?,
        sni: opt_string(obj.get("sni"))?,
        alpn: opt_string(obj.get("alpn"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_upsert() {
        let p = json!({
            "id": "n1",
            "name": "Demo",
            "server": "127.0.0.1",
            "port": 1080,
            "protocol": "socks5"
        });
        let r = parse_proxy_upsert_payload(Some(&p)).unwrap();
        assert_eq!(r.id, "n1");
        assert_eq!(r.port, 1080);
        assert_eq!(r.protocol, "socks5");
    }

    #[test]
    fn missing_payload() {
        assert_eq!(
            parse_proxy_upsert_payload(None).unwrap_err(),
            ProxyUpsertParseError::MissingPayload
        );
    }

    #[test]
    fn empty_payload() {
        assert_eq!(
            parse_proxy_upsert_payload(Some(&json!({}))).unwrap_err(),
            ProxyUpsertParseError::EmptyPayload
        );
    }

    #[test]
    fn malformed_port() {
        let p = json!({"id":"a","server":"h","port":"x"});
        assert_eq!(
            parse_proxy_upsert_payload(Some(&p)).unwrap_err(),
            ProxyUpsertParseError::MalformedPayload
        );
    }

    #[test]
    fn missing_server() {
        let p = json!({"id":"a","port":1080});
        assert_eq!(
            parse_proxy_upsert_payload(Some(&p)).unwrap_err(),
            ProxyUpsertParseError::MissingServer
        );
    }

    #[test]
    fn unknown_protocol() {
        let p = json!({"id":"a","server":"h","port":1,"protocol":"nope"});
        assert_eq!(
            parse_proxy_upsert_payload(Some(&p)).unwrap_err(),
            ProxyUpsertParseError::UnknownProtocol
        );
    }
}
