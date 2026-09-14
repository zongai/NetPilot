//! Reality / Vision / WS / gRPC / TLS parameter mapping (NP-134).

use std::collections::HashMap;

use netpilot_proxy::TransportKind;

pub fn map_transport_params(params: &HashMap<String, String>) -> Option<TransportKind> {
    let keys = ["type", "network", "transport"];
    for k in keys {
        if let Some(v) = params.get(k) {
            if let Some(t) = TransportKind::parse(v) {
                return Some(t);
            }
            match v.to_ascii_lowercase().as_str() {
                "ws" | "websocket" => return Some(TransportKind::Websocket),
                "grpc" => return Some(TransportKind::Grpc),
                "h2" | "http2" => return Some(TransportKind::Http2),
                "tcp" => return Some(TransportKind::Tcp),
                "udp" => return Some(TransportKind::Udp),
                "tls" => return Some(TransportKind::Tls),
                "reality" => return Some(TransportKind::Reality),
                _ => {}
            }
        }
    }
    if params.get("security").map(|s| s.eq_ignore_ascii_case("reality")) == Some(true)
        || params.contains_key("pbk")
        || params.contains_key("public-key")
    {
        return Some(TransportKind::Reality);
    }
    if params.get("security").map(|s| s.eq_ignore_ascii_case("tls")) == Some(true)
        || params.get("tls").map(|s| s == "true" || s == "1") == Some(true)
    {
        return Some(TransportKind::Tls);
    }
    if params
        .get("flow")
        .map(|f| f.to_ascii_lowercase().contains("vision"))
        .unwrap_or(false)
    {
        // Vision is a flow on VLESS; transport still typically TLS/REALITY.
        return Some(TransportKind::Tls);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ws_and_reality() {
        let mut p = HashMap::new();
        p.insert("type".into(), "ws".into());
        assert_eq!(map_transport_params(&p), Some(TransportKind::Websocket));
        p.clear();
        p.insert("security".into(), "reality".into());
        assert_eq!(map_transport_params(&p), Some(TransportKind::Reality));
    }
}
