//! RuleSet loader (NP-091).

use crate::parse::{parse_rules, ParseError};
use crate::ruleset::RuleSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Parse(ParseError),
    Empty,
    InvalidSource(&'static str),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "Parse: {e}"),
            Self::Empty => write!(f, "Empty"),
            Self::InvalidSource(m) => write!(f, "InvalidSource: {m}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<ParseError> for LoadError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

/// Load a RuleSet from inline text.
pub fn load_ruleset_from_str(id: &str, name: &str, text: &str) -> Result<RuleSet, LoadError> {
    if id.trim().is_empty() || name.trim().is_empty() {
        return Err(LoadError::InvalidSource("id/name required"));
    }
    let rules = parse_rules(text)?;
    if rules.is_empty() {
        return Err(LoadError::Empty);
    }
    Ok(RuleSet::new(id, name, rules))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_text() {
        let set = load_ruleset_from_str("d", "Default", "MATCH,DIRECT\n").unwrap();
        assert_eq!(set.len(), 1);
    }
}
