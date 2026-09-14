//! RuleSet model and process-aware matching (NP-089 / NP-090).

use crate::engine::RouteRequest;
use crate::parse::parse_rules;
use crate::{RouteDecision, RouteExplanation, Rule, RuleIndex, RuleMatcher};

/// Named, versioned rule collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSet {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub rules: Vec<Rule>,
}

impl RuleSet {
    pub fn new(id: impl Into<String>, name: impl Into<String>, rules: Vec<Rule>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: 1,
            rules,
        }
    }

    pub fn from_text(id: &str, name: &str, text: &str) -> Result<Self, crate::ParseError> {
        let rules = parse_rules(text)?;
        Ok(Self::new(id, name, rules))
    }

    pub fn index(&self) -> RuleIndex {
        RuleIndex::new(self.rules.clone())
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Matches traffic with optional process identity (NP-089).
pub fn match_with_process(
    index: &RuleIndex,
    host: Option<&str>,
    ip: Option<&str>,
    process_name: Option<&str>,
) -> Option<(RouteDecision, RouteExplanation)> {
    let mut req = RouteRequest::default();
    if let Some(h) = host {
        req = RouteRequest::domain(h);
    }
    if let Some(i) = ip {
        req.ip = Some(i.to_string());
    }
    if let Some(p) = process_name {
        req.process_name = Some(p.to_string());
    }
    index.match_request(&req)
}

/// Apply process-name rules preferentially when process is known.
pub fn process_rules_only(rules: &[Rule]) -> Vec<Rule> {
    rules
        .iter()
        .filter(|r| matches!(r.matcher, RuleMatcher::ProcessName(_)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteDecision;

    #[test]
    fn ruleset_from_text_and_process_match() {
        let set = RuleSet::from_text(
            "default",
            "Default",
            "PROCESS-NAME,chrome.exe,BROWSER\nMATCH,DIRECT\n",
        )
        .unwrap();
        assert_eq!(set.len(), 2);
        let idx = set.index();
        let (d, _) = match_with_process(&idx, Some("x.com"), None, Some("chrome.exe")).unwrap();
        assert_eq!(d.outbound, "BROWSER");
        let (d, _) = match_with_process(&idx, Some("x.com"), None, None).unwrap();
        assert_eq!(d.outbound, "DIRECT");
        let _ = RouteDecision::direct();
    }
}
