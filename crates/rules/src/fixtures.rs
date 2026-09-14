//! Rule fixtures and conformance cases (NP-048).

use crate::{
    parse_rules, RouteRequest, RoutingEngine, RuleIndex,
};

/// One expected routing outcome for a request.
#[derive(Debug, Clone)]
pub struct FixtureCase {
    pub name: &'static str,
    pub rules_text: &'static str,
    pub request: RouteRequest,
    pub expect_outbound: &'static str,
}

/// Built-in conformance suite (deterministic, no network).
pub fn conformance_cases() -> Vec<FixtureCase> {
    vec![
        FixtureCase {
            name: "domain_suffix_proxy",
            rules_text: "DOMAIN-SUFFIX,google.com,PROXY\nMATCH,DIRECT\n",
            request: RouteRequest::domain("www.google.com"),
            expect_outbound: "PROXY",
        },
        FixtureCase {
            name: "ip_cidr_direct",
            rules_text: "IP-CIDR,10.0.0.0/8,DIRECT\nMATCH,PROXY\n",
            request: RouteRequest::ip("10.1.2.3"),
            expect_outbound: "DIRECT",
        },
        FixtureCase {
            name: "port_match",
            rules_text: "PORT,443,HTTPS\nMATCH,DIRECT\n",
            request: {
                let mut r = RouteRequest::domain("x.com");
                r.port = Some(443);
                r
            },
            expect_outbound: "HTTPS",
        },
        FixtureCase {
            name: "process_match",
            rules_text: "PROCESS-NAME,chrome.exe,BROWSER\nMATCH,DIRECT\n",
            request: {
                let mut r = RouteRequest::domain("x.com");
                r.process_name = Some("chrome.exe".into());
                r
            },
            expect_outbound: "BROWSER",
        },
        FixtureCase {
            name: "network_udp",
            rules_text: "NETWORK,udp,UDP-OUT\nMATCH,DIRECT\n",
            request: {
                let mut r = RouteRequest::ip("1.1.1.1");
                r.network = Some(crate::NetworkProtocol::Udp);
                r
            },
            expect_outbound: "UDP-OUT",
        },
        FixtureCase {
            name: "reject_action",
            rules_text: "DOMAIN-KEYWORD,ads,REJECT\nMATCH,DIRECT\n",
            request: RouteRequest::domain("ads.tracker.io"),
            expect_outbound: "REJECT",
        },
        FixtureCase {
            name: "default_direct",
            rules_text: "DOMAIN,only.this,PROXY\n",
            request: RouteRequest::domain("other.test"),
            expect_outbound: "DIRECT",
        },
    ]
}

pub fn run_case(case: &FixtureCase) -> Result<(), String> {
    let rules = parse_rules(case.rules_text).map_err(|e| e.to_string())?;
    let engine = RoutingEngine::new(RuleIndex::new(rules));
    let (d, _) = engine.decide(&case.request);
    if d.outbound != case.expect_outbound {
        return Err(format!(
            "{}: expected {}, got {}",
            case.name, case.expect_outbound, d.outbound
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_conformance_cases_pass() {
        for case in conformance_cases() {
            run_case(&case).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}
