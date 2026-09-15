//! Routing engine entry (NP-198) — re-export façade.

pub use netpilot_rules::{
    RouteRequest, RoutingEngine, RuleIndex,
};

/// Convenience: build engine from raw rule text.
pub fn engine_from_text(text: &str) -> Result<RoutingEngine, netpilot_rules::ParseError> {
    let rules = netpilot_rules::parse_rules(text)?;
    Ok(RoutingEngine::new(RuleIndex::new(rules)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_direct() {
        let eng = engine_from_text("MATCH,DIRECT\n").unwrap();
        let (d, _) = eng.decide(&RouteRequest::domain("any.example"));
        assert_eq!(d.outbound, "DIRECT");
    }
}
