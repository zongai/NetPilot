//! Routing decision engine (NP-045) and explanation helpers (NP-046).

use crate::{RouteDecision, RouteExplanation, RuleIndex};

/// Traffic attributes available at decision time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouteRequest {
    pub domain: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub network: Option<crate::NetworkProtocol>,
    pub process_name: Option<String>,
}

impl RouteRequest {
    pub fn domain(host: impl Into<String>) -> Self {
        let host = host
            .into()
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        Self {
            domain: Some(host),
            ..Self::default()
        }
    }

    pub fn ip(addr: impl Into<String>) -> Self {
        Self {
            ip: Some(addr.into()),
            ..Self::default()
        }
    }

    pub fn primary_value(&self) -> Option<String> {
        self.domain
            .clone()
            .or_else(|| self.ip.clone())
            .or_else(|| self.process_name.clone())
            .or_else(|| self.port.map(|p| p.to_string()))
    }
}

/// Engine wraps a [`RuleIndex`] and always returns a decision (default DIRECT).
#[derive(Debug, Clone)]
pub struct RoutingEngine {
    index: RuleIndex,
    default: RouteDecision,
}

impl RoutingEngine {
    pub fn new(index: RuleIndex) -> Self {
        Self {
            index,
            default: RouteDecision::direct(),
        }
    }

    pub fn with_default(mut self, decision: RouteDecision) -> Self {
        self.default = decision;
        self
    }

    pub fn index(&self) -> &RuleIndex {
        &self.index
    }

    pub fn decide(&self, req: &RouteRequest) -> (RouteDecision, RouteExplanation) {
        if let Some((d, e)) = self.index.match_request(req) {
            return (d, e);
        }
        (
            self.default.clone(),
            RouteExplanation {
                rule_index: usize::MAX,
                matcher_kind: "default".into(),
                outbound: self.default.outbound.clone(),
                priority: i32::MAX,
                matched_value: req.primary_value(),
                source: None,
            },
        )
    }

    pub fn explain(&self, req: &RouteRequest) -> String {
        let (d, e) = self.decide(req);
        format!("{} | {}", e.summary(), d.outbound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_rules, RuleIndex};

    #[test]
    fn engine_default_when_no_match() {
        let engine = RoutingEngine::new(RuleIndex::new(vec![]));
        let (d, e) = engine.decide(&RouteRequest::domain("x.test"));
        assert_eq!(d.outbound, "DIRECT");
        assert_eq!(e.matcher_kind, "default");
    }

    #[test]
    fn engine_from_parsed_rules() {
        let rules = parse_rules("DOMAIN-SUFFIX,example.com,PROXY\nMATCH,DIRECT\n").unwrap();
        let engine = RoutingEngine::new(RuleIndex::new(rules));
        assert_eq!(
            engine
                .decide(&RouteRequest::domain("a.example.com"))
                .0
                .outbound,
            "PROXY"
        );
        assert!(engine.explain(&RouteRequest::domain("a.example.com")).contains("PROXY"));
    }
}
