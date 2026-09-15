//! `rules.decide` request schema (NP-INTEGRATION-001).
//!
//! Canonical payload:
//! ```json
//! { "domain": "www.google.com", "port": 443 }
//! ```
//! or
//! ```json
//! { "ip": "1.1.1.1", "port": 443 }
//! ```
//! At least one of `domain` / `ip` is required. `port` is optional (u16).

use serde_json::Value;

/// Parsed `rules.decide` request fields (owned, for handlers/tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesDecideRequest {
    pub domain: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u16>,
}

/// Stable validation errors mapped to `RouteError::InvalidInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesDecideParseError {
    MissingPayload,
    EmptyPayload,
    MalformedPayload,
    MissingTarget,
}

impl RulesDecideParseError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingPayload => "missing payload",
            Self::EmptyPayload => "empty payload",
            Self::MalformedPayload => "malformed payload",
            Self::MissingTarget => "payload.domain or payload.ip required",
        }
    }
}

/// Parse and validate a `rules.decide` payload object.
pub fn parse_rules_decide_payload(
    payload: Option<&Value>,
) -> Result<RulesDecideRequest, RulesDecideParseError> {
    let Some(payload) = payload else {
        return Err(RulesDecideParseError::MissingPayload);
    };
    if !payload.is_object() {
        return Err(RulesDecideParseError::MalformedPayload);
    }
    let obj = payload.as_object().unwrap();
    if obj.is_empty() {
        return Err(RulesDecideParseError::EmptyPayload);
    }

    let domain = match obj.get("domain") {
        None => None,
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Some(_) => return Err(RulesDecideParseError::MalformedPayload),
    };

    let ip = match obj.get("ip") {
        None => None,
        Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Some(_) => return Err(RulesDecideParseError::MalformedPayload),
    };

    let port = match obj.get("port") {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) => {
            let Some(u) = n.as_u64() else {
                return Err(RulesDecideParseError::MalformedPayload);
            };
            if u > u16::MAX as u64 {
                return Err(RulesDecideParseError::MalformedPayload);
            }
            Some(u as u16)
        }
        Some(_) => return Err(RulesDecideParseError::MalformedPayload),
    };

    if domain.is_none() && ip.is_none() {
        return Err(RulesDecideParseError::MissingTarget);
    }

    Ok(RulesDecideRequest { domain, ip, port })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_domain_port() {
        let p = json!({"domain": "www.google.com", "port": 443});
        let r = parse_rules_decide_payload(Some(&p)).unwrap();
        assert_eq!(r.domain.as_deref(), Some("www.google.com"));
        assert_eq!(r.port, Some(443));
        assert!(r.ip.is_none());
    }

    #[test]
    fn valid_ip_only() {
        let p = json!({"ip": "1.1.1.1"});
        let r = parse_rules_decide_payload(Some(&p)).unwrap();
        assert_eq!(r.ip.as_deref(), Some("1.1.1.1"));
    }

    #[test]
    fn missing_payload() {
        assert_eq!(
            parse_rules_decide_payload(None).unwrap_err(),
            RulesDecideParseError::MissingPayload
        );
    }

    #[test]
    fn empty_payload() {
        let p = json!({});
        assert_eq!(
            parse_rules_decide_payload(Some(&p)).unwrap_err(),
            RulesDecideParseError::EmptyPayload
        );
    }

    #[test]
    fn malformed_payload_array() {
        let p = json!([1, 2, 3]);
        assert_eq!(
            parse_rules_decide_payload(Some(&p)).unwrap_err(),
            RulesDecideParseError::MalformedPayload
        );
    }

    #[test]
    fn malformed_domain_type() {
        let p = json!({"domain": 123});
        assert_eq!(
            parse_rules_decide_payload(Some(&p)).unwrap_err(),
            RulesDecideParseError::MalformedPayload
        );
    }

    #[test]
    fn malformed_port_type() {
        let p = json!({"domain": "a.com", "port": "443"});
        assert_eq!(
            parse_rules_decide_payload(Some(&p)).unwrap_err(),
            RulesDecideParseError::MalformedPayload
        );
    }

    #[test]
    fn missing_target() {
        let p = json!({"port": 443});
        assert_eq!(
            parse_rules_decide_payload(Some(&p)).unwrap_err(),
            RulesDecideParseError::MissingTarget
        );
    }
}
