//! Rules engine: model, parse, match, route, explain, index (NP-037…NP-048).

#![forbid(unsafe_code)]

mod cache;
mod domain;
mod engine;
mod fixtures;
mod index;
mod integrity;
mod integration;
mod ip;
mod loader;
mod parse;
mod ruleset;
mod subscription;
mod updater;

pub use cache::RuleSetCache;
pub use domain::{domain_matches, DomainMatchKind};
pub use engine::{RouteRequest, RoutingEngine};
pub use fixtures::{conformance_cases, run_case, FixtureCase};
pub use index::IndexedRules;
pub use integrity::{content_fingerprint, verify_fingerprint, IntegrityError};
pub use integration::process_ruleset_smoke;
pub use ip::{ip_in_cidr, parse_cidr, parse_ip, Cidr, IpMatchError};
pub use loader::{load_ruleset_from_str, LoadError};
pub use parse::{parse_rule_line, parse_rules, ParseError};
pub use ruleset::{match_with_process, process_rules_only, RuleSet};
pub use subscription::{Subscription, SubscriptionManager, SubscriptionState};
pub use updater::RuleSetUpdater;

use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "netpilot-rules";

/// Network layer for port/protocol rules (NP-041).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    Any,
}

impl NetworkProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Any => "any",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "tcp" => Some(Self::Tcp),
            "udp" => Some(Self::Udp),
            "any" | "*" => Some(Self::Any),
            _ => None,
        }
    }

    pub fn matches(self, other: NetworkProtocol) -> bool {
        matches!(self, Self::Any) || matches!(other, Self::Any) || self == other
    }
}

/// How a rule matches traffic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleMatcher {
    Domain(String),
    DomainSuffix(String),
    DomainKeyword(String),
    IpCidr(String),
    /// Destination port (inclusive single port for skeleton).
    Port(u16),
    /// Network protocol filter.
    Network(NetworkProtocol),
    /// Process image name (case-insensitive basename), NP-042.
    ProcessName(String),
    MatchAll,
}

impl RuleMatcher {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Domain(_) => "domain",
            Self::DomainSuffix(_) => "domain_suffix",
            Self::DomainKeyword(_) => "domain_keyword",
            Self::IpCidr(_) => "ip_cidr",
            Self::Port(_) => "port",
            Self::Network(_) => "network",
            Self::ProcessName(_) => "process_name",
            Self::MatchAll => "match",
        }
    }
}

/// Normalized action kinds (NP-044).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleAction {
    Direct,
    Reject,
    /// Named proxy profile or group id.
    Proxy(String),
}

impl RuleAction {
    pub fn parse(outbound: &str) -> Self {
        match outbound.to_ascii_uppercase().as_str() {
            "DIRECT" => Self::Direct,
            "REJECT" | "BLOCK" => Self::Reject,
            _ => Self::Proxy(outbound.to_string()),
        }
    }

    pub fn outbound_name(&self) -> String {
        match self {
            Self::Direct => "DIRECT".into(),
            Self::Reject => "REJECT".into(),
            Self::Proxy(name) => name.clone(),
        }
    }
}

/// Routing action / outbound target name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    /// Logical outbound: `DIRECT`, `REJECT`, or a proxy/group id.
    pub outbound: String,
    pub action: RuleAction,
}

impl RouteDecision {
    pub fn from_outbound(outbound: impl Into<String>) -> Self {
        let outbound = outbound.into();
        let action = RuleAction::parse(&outbound);
        Self {
            outbound: action.outbound_name(),
            action,
        }
    }

    pub fn direct() -> Self {
        Self::from_outbound("DIRECT")
    }

    pub fn reject() -> Self {
        Self::from_outbound("REJECT")
    }

    pub fn proxy(name: impl Into<String>) -> Self {
        Self::from_outbound(name)
    }
}

/// Single rule: matcher + decision + explicit priority (NP-043).
///
/// Lower `priority` value is evaluated first. Rules with equal priority keep
/// insertion order (stable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub matcher: RuleMatcher,
    pub decision: RouteDecision,
    /// Explicit priority; default 1000. Lower runs first.
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Original source line (for diagnostics; no secrets expected).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

fn default_priority() -> i32 {
    1000
}

/// Why a rule was chosen (NP-046).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteExplanation {
    pub rule_index: usize,
    pub matcher_kind: String,
    pub outbound: String,
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl RouteExplanation {
    pub fn summary(&self) -> String {
        format!(
            "rule#{} {} -> {} (priority {})",
            self.rule_index, self.matcher_kind, self.outbound, self.priority
        )
    }
}

/// Ordered rule list with sequential evaluation (stable sort by priority).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleIndex {
    rules: Vec<Rule>,
}

impl RuleIndex {
    pub fn new(mut rules: Vec<Rule>) -> Self {
        // Deterministic priority: lower priority number first; stable for ties.
        let mut indexed: Vec<(usize, Rule)> = rules.drain(..).enumerate().collect();
        indexed.sort_by(|a, b| a.1.priority.cmp(&b.1.priority).then_with(|| a.0.cmp(&b.0)));
        Self {
            rules: indexed.into_iter().map(|(_, r)| r).collect(),
        }
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

    /// Evaluate against a full request context (NP-045).
    pub fn match_request(&self, req: &RouteRequest) -> Option<(RouteDecision, RouteExplanation)> {
        for (idx, rule) in self.rules.iter().enumerate() {
            if rule_matches(rule, req) {
                return Some((
                    rule.decision.clone(),
                    RouteExplanation {
                        rule_index: idx,
                        matcher_kind: rule.matcher.kind_name().into(),
                        outbound: rule.decision.outbound.clone(),
                        priority: rule.priority,
                        matched_value: req.primary_value(),
                        source: rule.source.clone(),
                    },
                ));
            }
        }
        None
    }

    pub fn match_domain(&self, host: &str) -> Option<(RouteDecision, RouteExplanation)> {
        self.match_request(&RouteRequest::domain(host))
    }

    pub fn match_ip(&self, ip: &str) -> Option<(RouteDecision, RouteExplanation)> {
        self.match_request(&RouteRequest::ip(ip))
    }
}

fn rule_matches(rule: &Rule, req: &RouteRequest) -> bool {
    match &rule.matcher {
        RuleMatcher::Domain(d) => req
            .domain
            .as_ref()
            .map(|h| domain_matches(DomainMatchKind::Exact, d, h))
            .unwrap_or(false),
        RuleMatcher::DomainSuffix(d) => req
            .domain
            .as_ref()
            .map(|h| domain_matches(DomainMatchKind::Suffix, d, h))
            .unwrap_or(false),
        RuleMatcher::DomainKeyword(k) => req
            .domain
            .as_ref()
            .map(|h| domain_matches(DomainMatchKind::Keyword, k, h))
            .unwrap_or(false),
        RuleMatcher::IpCidr(cidr) => req
            .ip
            .as_ref()
            .map(|ip| ip_in_cidr(ip, cidr).unwrap_or(false))
            .unwrap_or(false),
        RuleMatcher::Port(p) => req.port == Some(*p),
        RuleMatcher::Network(proto) => req.network.map(|n| proto.matches(n)).unwrap_or(false),
        RuleMatcher::ProcessName(name) => req
            .process_name
            .as_ref()
            .map(|p| process_name_matches(name, p))
            .unwrap_or(false),
        RuleMatcher::MatchAll => true,
    }
}

fn process_name_matches(pattern: &str, process: &str) -> bool {
    let pattern = pattern.trim().to_ascii_lowercase();
    let process = process.trim().to_ascii_lowercase();
    // basename match: `chrome.exe` matches `C:\...\chrome.exe`
    let base = process
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(process.as_str());
    base == pattern || process == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_suffix_match() {
        let idx = RuleIndex::new(vec![Rule {
            matcher: RuleMatcher::DomainSuffix("google.com".into()),
            decision: RouteDecision::proxy("PROXY"),
            priority: 1000,
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
            priority: 1000,
            source: None,
        }]);
        assert_eq!(idx.match_ip("10.1.2.3").unwrap().0.outbound, "DIRECT");
        assert!(idx.match_ip("11.0.0.1").is_none());
    }

    #[test]
    fn priority_orders_rules() {
        let idx = RuleIndex::new(vec![
            Rule {
                matcher: RuleMatcher::MatchAll,
                decision: RouteDecision::direct(),
                priority: 100,
                source: None,
            },
            Rule {
                matcher: RuleMatcher::DomainSuffix("google.com".into()),
                decision: RouteDecision::proxy("PROXY"),
                priority: 10,
                source: None,
            },
        ]);
        assert_eq!(
            idx.match_domain("www.google.com").unwrap().0.outbound,
            "PROXY"
        );
    }

    #[test]
    fn port_and_process() {
        let idx = RuleIndex::new(vec![
            Rule {
                matcher: RuleMatcher::Port(443),
                decision: RouteDecision::proxy("HTTPS"),
                priority: 10,
                source: None,
            },
            Rule {
                matcher: RuleMatcher::ProcessName("chrome.exe".into()),
                decision: RouteDecision::proxy("BROWSER"),
                priority: 20,
                source: None,
            },
        ]);
        let mut req = RouteRequest::domain("x.com");
        req.port = Some(443);
        assert_eq!(idx.match_request(&req).unwrap().0.outbound, "HTTPS");
        let mut req = RouteRequest::domain("x.com");
        req.process_name = Some(r"C:\Program Files\Google\Chrome\chrome.exe".into());
        assert_eq!(idx.match_request(&req).unwrap().0.outbound, "BROWSER");
    }
}
