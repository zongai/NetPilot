//! Rule line parser (NP-038).

use crate::{RouteDecision, Rule, RuleMatcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidSyntax(String),
    UnknownType(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::InvalidSyntax(s) => write!(f, "InvalidSyntax: {s}"),
            Self::UnknownType(t) => write!(f, "UnknownType: {t}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse one rule line (Clash-like):
/// `DOMAIN,example.com,DIRECT`
/// `DOMAIN-SUFFIX,google.com,PROXY`
/// `DOMAIN-KEYWORD,ads,REJECT`
/// `IP-CIDR,10.0.0.0/8,DIRECT`
/// `MATCH,PROXY`
pub fn parse_rule_line(line: &str) -> Result<Option<Rule>, ParseError> {
    let raw = line.trim();
    if raw.is_empty() || raw.starts_with('#') {
        return Ok(None);
    }
    let parts: Vec<&str> = raw.split(',').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return Err(ParseError::InvalidSyntax(raw.to_string()));
    }
    let type_name = parts[0].to_ascii_uppercase();
    let (matcher, outbound) = match type_name.as_str() {
        "DOMAIN" => {
            if parts.len() < 3 {
                return Err(ParseError::InvalidSyntax(raw.to_string()));
            }
            (RuleMatcher::Domain(parts[1].to_ascii_lowercase()), parts[2])
        }
        "DOMAIN-SUFFIX" => {
            if parts.len() < 3 {
                return Err(ParseError::InvalidSyntax(raw.to_string()));
            }
            (
                RuleMatcher::DomainSuffix(parts[1].to_ascii_lowercase()),
                parts[2],
            )
        }
        "DOMAIN-KEYWORD" => {
            if parts.len() < 3 {
                return Err(ParseError::InvalidSyntax(raw.to_string()));
            }
            (
                RuleMatcher::DomainKeyword(parts[1].to_ascii_lowercase()),
                parts[2],
            )
        }
        "IP-CIDR" | "IP-CIDR6" => {
            if parts.len() < 3 {
                return Err(ParseError::InvalidSyntax(raw.to_string()));
            }
            (RuleMatcher::IpCidr(parts[1].to_string()), parts[2])
        }
        "MATCH" => (RuleMatcher::MatchAll, parts[1]),
        other => return Err(ParseError::UnknownType(other.to_string())),
    };
    if outbound.is_empty() {
        return Err(ParseError::InvalidSyntax(raw.to_string()));
    }
    Ok(Some(Rule {
        matcher,
        decision: RouteDecision::proxy(outbound),
        source: Some(raw.to_string()),
    }))
}

pub fn parse_rules(text: &str) -> Result<Vec<Rule>, ParseError> {
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(rule) = parse_rule_line(line)? {
            out.push(rule);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_domain_suffix() {
        let r = parse_rule_line("DOMAIN-SUFFIX,Google.COM,PROXY")
            .unwrap()
            .unwrap();
        assert_eq!(r.matcher, RuleMatcher::DomainSuffix("google.com".into()));
        assert_eq!(r.decision.outbound, "PROXY");
    }

    #[test]
    fn parse_match_and_comments() {
        assert!(parse_rule_line("# comment").unwrap().is_none());
        let r = parse_rule_line("MATCH,DIRECT").unwrap().unwrap();
        assert_eq!(r.matcher, RuleMatcher::MatchAll);
    }

    #[test]
    fn parse_block() {
        let text = "DOMAIN,example.com,DIRECT\nIP-CIDR,192.168.0.0/16,DIRECT\n";
        let rules = parse_rules(text).unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn unknown_type() {
        assert!(matches!(
            parse_rule_line("FOO,bar,DIRECT"),
            Err(ParseError::UnknownType(_))
        ));
    }
}
