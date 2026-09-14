//! Routing engine surface (re-exports rules; engine entry for Core).

#![forbid(unsafe_code)]

pub use netpilot_rules::{
    conformance_cases, domain_matches, ip_in_cidr, parse_cidr, parse_ip, parse_rule_line,
    parse_rules, run_case, Cidr, DomainMatchKind, FixtureCase, IndexedRules, IpMatchError,
    NetworkProtocol, ParseError, RouteDecision, RouteExplanation, RouteRequest, RoutingEngine,
    Rule, RuleAction, RuleIndex, RuleMatcher,
};

pub const CRATE_NAME: &str = "netpilot-routing";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_parse_and_match() {
        let rules = parse_rules(
            "DOMAIN-SUFFIX,google.com,PROXY\nIP-CIDR,10.0.0.0/8,DIRECT\nMATCH,DIRECT\n",
        )
        .unwrap();
        let engine = RoutingEngine::new(RuleIndex::new(rules));
        assert_eq!(
            engine
                .decide(&RouteRequest::domain("mail.google.com"))
                .0
                .outbound,
            "PROXY"
        );
        assert_eq!(
            engine.decide(&RouteRequest::ip("10.9.9.9")).0.outbound,
            "DIRECT"
        );
    }

    #[test]
    fn conformance_via_routing() {
        for case in conformance_cases() {
            run_case(&case).unwrap();
        }
    }
}
