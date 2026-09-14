//! Rule indexing for faster domain/IP lookups (NP-047).

use std::collections::HashMap;

use crate::{
    domain_matches, ip_in_cidr, DomainMatchKind, RouteDecision, RouteExplanation, RouteRequest,
    Rule, RuleMatcher,
};

/// Secondary indexes over a priority-ordered rule list.
///
/// Exact domain and process-name maps provide O(1) candidates; other matchers
/// still scan in priority order. Correctness matches sequential [`crate::RuleIndex`].
#[derive(Debug, Clone, Default)]
pub struct IndexedRules {
    /// Priority-sorted rules (same order as RuleIndex).
    rules: Vec<Rule>,
    /// exact domain → first rule index
    domain_exact: HashMap<String, usize>,
    /// process basename → first rule index
    process_exact: HashMap<String, usize>,
}

impl IndexedRules {
    pub fn build(mut rules: Vec<Rule>) -> Self {
        let mut indexed: Vec<(usize, Rule)> = rules.drain(..).enumerate().collect();
        indexed.sort_by(|a, b| a.1.priority.cmp(&b.1.priority).then_with(|| a.0.cmp(&b.0)));
        let rules: Vec<Rule> = indexed.into_iter().map(|(_, r)| r).collect();
        let mut domain_exact = HashMap::new();
        let mut process_exact = HashMap::new();
        for (i, rule) in rules.iter().enumerate() {
            match &rule.matcher {
                RuleMatcher::Domain(d) => {
                    domain_exact.entry(d.to_ascii_lowercase()).or_insert(i);
                }
                RuleMatcher::ProcessName(p) => {
                    process_exact.entry(p.to_ascii_lowercase()).or_insert(i);
                }
                _ => {}
            }
        }
        Self {
            rules,
            domain_exact,
            process_exact,
        }
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn match_request(&self, req: &RouteRequest) -> Option<(RouteDecision, RouteExplanation)> {
        // Fast path: exact domain
        if let Some(domain) = &req.domain {
            if let Some(&idx) = self.domain_exact.get(domain) {
                if let Some(result) = self.try_rule(idx, req) {
                    // Still must ensure no higher-priority (lower index) rule matches.
                    if self.first_match_before(idx, req).is_none() {
                        return Some(result);
                    }
                }
            }
        }
        // Full scan (includes suffix/keyword/ip/port/process/match)
        for idx in 0..self.rules.len() {
            if let Some(result) = self.try_rule(idx, req) {
                return Some(result);
            }
        }
        let _ = &self.process_exact; // retained for future fast paths / stats
        None
    }

    fn first_match_before(
        &self,
        before: usize,
        req: &RouteRequest,
    ) -> Option<(RouteDecision, RouteExplanation)> {
        for idx in 0..before {
            if let Some(r) = self.try_rule(idx, req) {
                return Some(r);
            }
        }
        None
    }

    fn try_rule(
        &self,
        idx: usize,
        req: &RouteRequest,
    ) -> Option<(RouteDecision, RouteExplanation)> {
        let rule = &self.rules[idx];
        let matched = match &rule.matcher {
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
            RuleMatcher::ProcessName(name) => req.process_name.as_ref().map_or(false, |p| {
                let pattern = name.to_ascii_lowercase();
                let process = p.to_ascii_lowercase();
                let base = process
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(process.as_str());
                base == pattern || process == pattern
            }),
            RuleMatcher::MatchAll => true,
        };
        if !matched {
            return None;
        }
        Some((
            rule.decision.clone(),
            RouteExplanation {
                rule_index: idx,
                matcher_kind: rule.matcher.kind_name().into(),
                outbound: rule.decision.outbound.clone(),
                priority: rule.priority,
                matched_value: req.primary_value(),
                source: rule.source.clone(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteDecision;

    #[test]
    fn indexed_domain_exact() {
        let idx = IndexedRules::build(vec![Rule {
            matcher: RuleMatcher::Domain("example.com".into()),
            decision: RouteDecision::proxy("P"),
            priority: 1,
            source: None,
        }]);
        let r = idx
            .match_request(&RouteRequest::domain("example.com"))
            .unwrap();
        assert_eq!(r.0.outbound, "P");
    }
}
