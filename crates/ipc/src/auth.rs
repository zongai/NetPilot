//! Local IPC authorization boundary (NP-022).

/// Identity of a local peer (Desktop or tool). No secrets stored here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    /// Process id when known (0 = unknown).
    pub pid: u32,
    /// Executable path hint (may be empty); never log if it might embed secrets.
    pub exe_path: String,
    /// Whether the peer is treated as the primary UI.
    pub is_desktop: bool,
}

impl PeerIdentity {
    pub fn unknown() -> Self {
        Self {
            pid: 0,
            exe_path: String::new(),
            is_desktop: false,
        }
    }

    pub fn desktop(pid: u32, exe_path: impl Into<String>) -> Self {
        Self {
            pid,
            exe_path: exe_path.into(),
            is_desktop: true,
        }
    }
}

/// Operations that require elevated trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    /// Read-only status / health.
    Read,
    /// Mutate config / proxy selection.
    Write,
    /// TUN / privileged network changes.
    Privileged,
}

/// Authorization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    Allow,
    Deny { reason: &'static str },
}

/// Local-only policy: Desktop may Read/Write; Privileged needs explicit flag.
#[derive(Debug, Clone, Default)]
pub struct LocalAuthPolicy {
    pub allow_privileged_from_desktop: bool,
}

impl LocalAuthPolicy {
    pub fn authorize(&self, peer: &PeerIdentity, privilege: Privilege) -> AuthDecision {
        match privilege {
            Privilege::Read => AuthDecision::Allow,
            Privilege::Write => {
                if peer.is_desktop || peer.pid != 0 {
                    AuthDecision::Allow
                } else {
                    AuthDecision::Deny {
                        reason: "write requires identified local peer",
                    }
                }
            }
            Privilege::Privileged => {
                if peer.is_desktop && self.allow_privileged_from_desktop {
                    AuthDecision::Allow
                } else {
                    AuthDecision::Deny {
                        reason: "privileged operation not permitted for peer",
                    }
                }
            }
        }
    }
}

/// Map operation name to required privilege (stable defaults).
pub fn privilege_for_operation(operation: &str) -> Privilege {
    if operation.starts_with("health.") || operation.starts_with("runtime.") {
        Privilege::Read
    } else if operation.starts_with("tun.") || operation.starts_with("system.") {
        Privilege::Privileged
    } else {
        Privilege::Write
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_can_write() {
        let policy = LocalAuthPolicy::default();
        let peer = PeerIdentity::desktop(42, "NetPilot.Desktop.exe");
        assert_eq!(
            policy.authorize(&peer, Privilege::Write),
            AuthDecision::Allow
        );
    }

    #[test]
    fn privileged_denied_by_default() {
        let policy = LocalAuthPolicy::default();
        let peer = PeerIdentity::desktop(1, "ui");
        assert!(matches!(
            policy.authorize(&peer, Privilege::Privileged),
            AuthDecision::Deny { .. }
        ));
    }

    #[test]
    fn privileged_allowed_when_enabled() {
        let policy = LocalAuthPolicy {
            allow_privileged_from_desktop: true,
        };
        let peer = PeerIdentity::desktop(1, "ui");
        assert_eq!(
            policy.authorize(&peer, Privilege::Privileged),
            AuthDecision::Allow
        );
    }

    #[test]
    fn operation_mapping() {
        assert_eq!(privilege_for_operation("health.check"), Privilege::Read);
        assert_eq!(privilege_for_operation("tun.start"), Privilege::Privileged);
        assert_eq!(privilege_for_operation("config.apply"), Privilege::Write);
    }
}
