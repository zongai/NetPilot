//! Proxy group model and selection (NP-031).

use crate::{GroupSelect, ProxyGroup};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupError {
    EmptyMembers,
    UnknownMember(String),
    NoHealthyMember,
    InvalidSelection(String),
}

impl std::fmt::Display for GroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyMembers => write!(f, "EmptyMembers"),
            Self::UnknownMember(id) => write!(f, "UnknownMember: {id}"),
            Self::NoHealthyMember => write!(f, "NoHealthyMember"),
            Self::InvalidSelection(id) => write!(f, "InvalidSelection: {id}"),
        }
    }
}

impl std::error::Error for GroupError {}

/// Latency sample for UrlTest ranking (milliseconds; None = unreachable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberHealth {
    pub latency_ms: Option<u32>,
}

impl MemberHealth {
    pub fn healthy(latency_ms: u32) -> Self {
        Self {
            latency_ms: Some(latency_ms),
        }
    }

    pub fn down() -> Self {
        Self { latency_ms: None }
    }

    pub fn is_up(self) -> bool {
        self.latency_ms.is_some()
    }
}

/// Resolve which member id should be active for a group.
pub fn select_member(
    group: &ProxyGroup,
    health: &[(&str, MemberHealth)],
) -> Result<String, GroupError> {
    if group.members.is_empty() {
        return Err(GroupError::EmptyMembers);
    }

    match group.select {
        GroupSelect::Manual => {
            if let Some(sel) = &group.selected {
                if group.members.iter().any(|m| m == sel) {
                    return Ok(sel.clone());
                }
                return Err(GroupError::InvalidSelection(sel.clone()));
            }
            Ok(group.members[0].clone())
        }
        GroupSelect::Fallback => {
            for m in &group.members {
                let up = health
                    .iter()
                    .find(|(id, _)| *id == m)
                    .map(|(_, h)| h.is_up())
                    .unwrap_or(true);
                if up {
                    return Ok(m.clone());
                }
            }
            Err(GroupError::NoHealthyMember)
        }
        GroupSelect::UrlTest => {
            let mut best: Option<(String, u32)> = None;
            for m in &group.members {
                let lat = health
                    .iter()
                    .find(|(id, _)| *id == m)
                    .and_then(|(_, h)| h.latency_ms);
                if let Some(ms) = lat {
                    match best {
                        Some((_, b)) if ms >= b => {}
                        _ => best = Some((m.clone(), ms)),
                    }
                }
            }
            best.map(|(id, _)| id).ok_or(GroupError::NoHealthyMember)
        }
    }
}

/// Apply selection result onto the group.
pub fn apply_selection(group: &mut ProxyGroup, member_id: &str) -> Result<(), GroupError> {
    if !group.members.iter().any(|m| m == member_id) {
        return Err(GroupError::UnknownMember(member_id.to_string()));
    }
    group.selected = Some(member_id.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(select: GroupSelect, members: &[&str], selected: Option<&str>) -> ProxyGroup {
        ProxyGroup {
            id: "g".into(),
            name: "g".into(),
            select,
            members: members.iter().map(|s| (*s).to_string()).collect(),
            selected: selected.map(|s| s.to_string()),
        }
    }

    #[test]
    fn manual_uses_selected() {
        let g = group(GroupSelect::Manual, &["a", "b"], Some("b"));
        assert_eq!(select_member(&g, &[]).unwrap(), "b");
    }

    #[test]
    fn fallback_skips_down() {
        let g = group(GroupSelect::Fallback, &["a", "b"], None);
        let health = [
            ("a", MemberHealth::down()),
            ("b", MemberHealth::healthy(10)),
        ];
        assert_eq!(select_member(&g, &health).unwrap(), "b");
    }

    #[test]
    fn url_test_picks_lowest_latency() {
        let g = group(GroupSelect::UrlTest, &["a", "b", "c"], None);
        let health = [
            ("a", MemberHealth::healthy(50)),
            ("b", MemberHealth::healthy(20)),
            ("c", MemberHealth::down()),
        ];
        assert_eq!(select_member(&g, &health).unwrap(), "b");
    }

    #[test]
    fn apply_selection_rejects_unknown() {
        let mut g = group(GroupSelect::Manual, &["a"], None);
        assert!(matches!(
            apply_selection(&mut g, "x"),
            Err(GroupError::UnknownMember(_))
        ));
    }
}
