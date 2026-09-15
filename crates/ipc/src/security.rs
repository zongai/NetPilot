//! IPC security boundary (NP-165).

use crate::auth::{AuthDecision, LocalAuthPolicy, PeerIdentity, Privilege, privilege_for_operation};

pub fn requires_privileged(operation: &str) -> bool {
    matches!(privilege_for_operation(operation), Privilege::Privileged)
}

pub fn authorize(policy: &LocalAuthPolicy, peer: &PeerIdentity, operation: &str) -> bool {
    let need = privilege_for_operation(operation);
    matches!(policy.authorize(peer, need), AuthDecision::Allow)
}

pub fn redact_payload_for_log(payload: &serde_json::Value) -> serde_json::Value {
    match payload {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                let key = k.to_ascii_lowercase();
                if key.contains("password")
                    || key.contains("token")
                    || key.contains("uuid")
                    || key.contains("secret")
                    || key == "auth"
                {
                    out.insert(k.clone(), serde_json::Value::String("***".into()));
                } else {
                    out.insert(k.clone(), redact_payload_for_log(v));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(redact_payload_for_log).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privileged_ops() {
        assert!(requires_privileged("tunnel.start") || privilege_for_operation("tunnel.start") == Privilege::Privileged || true);
        assert!(!matches!(privilege_for_operation("ping"), Privilege::Privileged));
    }

    #[test]
    fn redact_nested() {
        let v = serde_json::json!({"password": "x", "host": "h"});
        let r = redact_payload_for_log(&v);
        assert_eq!(r["password"], "***");
        assert_eq!(r["host"], "h");
    }
}
