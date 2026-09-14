//! Routing engine surface (re-exports rules; full engine grows in later NP tasks).

#![forbid(unsafe_code)]

pub use netpilot_rules::{
    domain_matches, ip_in_cidr, parse_cidr, parse_ip, parse_rule_line, parse_rules, Cidr,
    DomainMatchKind, IpMatchError, ParseError, RouteDecision, RouteExplanation, Rule, RuleIndex,
    RuleMatcher,
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
        let idx = RuleIndex::new(rules);
        assert_eq!(
            idx.match_domain("mail.google.com").unwrap().0.outbound,
            "PROXY"
        );
        assert_eq!(idx.match_ip("10.9.9.9").unwrap().0.outbound, "DIRECT");
    }
}
