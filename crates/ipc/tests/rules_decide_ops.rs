//! Integration-style checks for rules.decide schema + router mapping.

use netpilot_ipc::{
    echo_handler, map_route_error, parse_rules_decide_payload, IpcEnvelope, RequestRouter,
    RouteError, RouteOutcome, RulesDecideParseError,
};
use serde_json::json;

#[test]
fn valid_rules_decide_request() {
    let p = json!({"domain": "www.google.com", "port": 443});
    let r = parse_rules_decide_payload(Some(&p)).expect("valid");
    assert_eq!(r.domain.as_deref(), Some("www.google.com"));
    assert_eq!(r.port, Some(443));
}

#[test]
fn missing_payload() {
    let err = parse_rules_decide_payload(None).unwrap_err();
    assert_eq!(err, RulesDecideParseError::MissingPayload);
    let env = map_route_error(
        "1",
        Some("rules.decide".into()),
        &RouteError::InvalidInput(err.as_str()),
    );
    assert!(env
        .error
        .as_ref()
        .unwrap()
        .message
        .contains("missing payload"));
    assert_eq!(env.error.as_ref().unwrap().kind, "invalid_input");
}

#[test]
fn empty_payload() {
    let err = parse_rules_decide_payload(Some(&json!({}))).unwrap_err();
    assert_eq!(err, RulesDecideParseError::EmptyPayload);
    let env = map_route_error(
        "1",
        Some("rules.decide".into()),
        &RouteError::InvalidInput(err.as_str()),
    );
    assert_eq!(env.error.as_ref().unwrap().kind, "invalid_input");
}

#[test]
fn malformed_payload() {
    let err = parse_rules_decide_payload(Some(&json!([true]))).unwrap_err();
    assert_eq!(err, RulesDecideParseError::MalformedPayload);
    let env = map_route_error(
        "1",
        Some("rules.decide".into()),
        &RouteError::InvalidInput(err.as_str()),
    );
    assert_eq!(env.error.as_ref().unwrap().code, Some(400));
}

#[test]
fn unknown_operation() {
    let mut router = RequestRouter::new();
    router.register("echo", echo_handler);
    let req = IpcEnvelope::request("rid", "rules.decide.unknown");
    match router.dispatch(&req).unwrap() {
        RouteOutcome::NotFound { operation } => assert_eq!(operation, "rules.decide.unknown"),
        other => panic!("expected NotFound, got {other:?}"),
    }
}
