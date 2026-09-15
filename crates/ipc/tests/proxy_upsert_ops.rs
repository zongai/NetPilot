use netpilot_ipc::{
    map_route_error, parse_proxy_upsert_payload, ProxyUpsertParseError, RouteError,
};
use serde_json::json;

#[test]
fn valid_proxy_upsert_request() {
    let p = json!({
        "id": "demo-socks5",
        "name": "Demo SOCKS5",
        "server": "127.0.0.1",
        "port": 1080,
        "protocol": "socks5"
    });
    let r = parse_proxy_upsert_payload(Some(&p)).unwrap();
    assert_eq!(r.id, "demo-socks5");
    assert_eq!(r.server, "127.0.0.1");
    assert_eq!(r.port, 1080);
}

#[test]
fn missing_payload() {
    let err = parse_proxy_upsert_payload(None).unwrap_err();
    assert_eq!(err, ProxyUpsertParseError::MissingPayload);
    let env = map_route_error(
        "1",
        Some("proxy.upsert".into()),
        &RouteError::InvalidInput(err.as_str()),
    );
    assert_eq!(env.error.as_ref().unwrap().kind, "invalid_input");
}

#[test]
fn empty_payload() {
    assert_eq!(
        parse_proxy_upsert_payload(Some(&json!({}))).unwrap_err(),
        ProxyUpsertParseError::EmptyPayload
    );
}

#[test]
fn malformed_payload() {
    assert_eq!(
        parse_proxy_upsert_payload(Some(&json!([]))).unwrap_err(),
        ProxyUpsertParseError::MalformedPayload
    );
}

#[test]
fn missing_id() {
    let p = json!({"server": "127.0.0.1", "port": 1080});
    assert_eq!(
        parse_proxy_upsert_payload(Some(&p)).unwrap_err(),
        ProxyUpsertParseError::MissingId
    );
}
