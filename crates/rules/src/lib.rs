//! Rule domain model, parser, domain/IP matchers (NP-037…NP-040).

#![forbid(unsafe_code)]

mod domain;
mod ip;
mod parse;

pub use domain::{domain_matches, DomainMatchKind};
pub use ip::{ip_in_cidr, parse_cidr, parse_ip, Cidr, IpMatchError};
pub use parse::{parse_rule_line, parse_rules, ParseError};

use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "netpilot-rules";

/// How a rule matches traffic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleMatcher {
    Domain(String),
    DomainSuffix(String),
    DomainKeyword(String),
    IpCidr(String),
    MatchAll,
}

impl RuleMatcher {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Domain(_) => "domain",
            Self::DomainSuffix(_) => "domain_suffix",
            Self::DomainKeyword(_) => "domain_keyword",
            Self::IpCidr(_) => "ip_cidr",
            Self::MatchAll => "match",
        }
    }
}

/// Routing action / outbound target name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    /// Logical outbound: `DIRECT`, `REJECT`, or a proxy/group id.
    pub outbound: String,
}

impl RouteDecision {
    pub fn direct() -> Self {
        Self {
            outbound: "DIRECT".into(),
        }
    }

    pub fn reject() -> Self {
        Self {
            outbound: "REJECT".into(),
        }
    }

    pub fn proxy(name: impl Into<String>) -> Self {
        Self {
            outbound: name.into(),
        }
    }
}

/// Single rule: matcher + decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub matcher: RuleMatcher,
    pub decision: RouteDecision,
    /// Original source line (for diagnostics; no secrets expected).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Why a rule was chosen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteExplanation {
    pub rule_index: usize,
    pub matcher_kind: String,
    pub outbound: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_value: Option<String>,
}

/// Ordered rule list with sequential evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleIndex {
    rules: Vec<Rule>,
}

impl RuleIndex {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Evaluate domain-oriented rules then IP rules; first match wins.
    pub fn match_domain(&self, host: &str) -> Option<(RouteDecision, RouteExplanation)> {
        let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
        for (idx, rule) in self.rules.iter().enumerate() {
            let matched = match &rule.matcher {
                RuleMatcher::Domain(d) => domain_matches(DomainMatchKind::Exact, d, &host),
                RuleMatcher::DomainSuffix(d) => domain_matches(DomainMatchKind::Suffix, d, &host),
                RuleMatcher::DomainKeyword(k) => domain_matches(DomainMatchKind::Keyword, k, &host),
                RuleMatcher::MatchAll => true,
                RuleMatcher::IpCidr(_) => false,
            };
            if matched {
                return Some((
                    rule.decision.clone(),
                    RouteExplanation {
                        rule_index: idx,
                        matcher_kind: rule.matcher.kind_name().into(),
                        outbound: rule.decision.outbound.clone(),
                        matched_value: Some(host),
                    },
                ));
            }
        }
        None
    }

    pub fn match_ip(&self, ip: &str) -> Option<(RouteDecision, RouteExplanation)> {
        for (idx, rule) in self.rules.iter().enumerate() {
            let matched = match &rule.matcher {
                RuleMatcher::IpCidr(cidr) => ip_in_cidr(ip, cidr).unwrap_or(false),
                RuleMatcher::MatchAll => true,
                _ => false,
            };
            if matched {
                return Some((
                    rule.decision.clone(),
                    RouteExplanation {
                        rule_index: idx,
                        matcher_kind: rule.matcher.kind_name().into(),
                        outbound: rule.decision.outbound.clone(),
                        matched_value: Some(ip.to_string()),
                    },
                ));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_suffix_match() {
        let idx = RuleIndex::new(vec![Rule {
            matcher: RuleMatcher::DomainSuffix("google.com".into()),
            decision: RouteDecision::proxy("PROXY"),
            source: None,
        }]);
        let (d, exp) = idx.match_domain("www.google.com").unwrap();
        assert_eq!(d.outbound, "PROXY");
        assert_eq!(exp.rule_index, 0);
        assert!(idx.match_domain("example.com").is_none());
    }

    #[test]
    fn ip_cidr_match() {
        let idx = RuleIndex::new(vec![Rule {
            matcher: RuleMatcher::IpCidr("10.0.0.0/8".into()),
            decision: RouteDecision::direct(),
            source: None,
        }]);
        assert_eq!(idx.match_ip("10.1.2.3").unwrap().0.outbound, "DIRECT");
        assert!(idx.match_ip("11.0.0.1").is_none());
    }
}
