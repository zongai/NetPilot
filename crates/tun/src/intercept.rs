//! TCP / UDP interception path hooks (NP-068 / NP-069).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterceptContext {
    pub src: String,
    pub dst: String,
    pub protocol: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterceptAction {
    /// Deliver to userspace stack / proxy path.
    Capture,
    /// Write back to TUN / OS stack unchanged.
    Pass,
    /// Drop silently.
    Drop,
}

/// TCP interception policy (skeleton: capture non-local).
#[derive(Debug, Default, Clone)]
pub struct TcpIntercept {
    pub capture_all: bool,
}

impl TcpIntercept {
    pub fn inspect(&self, ctx: &InterceptContext) -> InterceptAction {
        if ctx.protocol != "tcp" {
            return InterceptAction::Pass;
        }
        if self.capture_all {
            return InterceptAction::Capture;
        }
        // Default: capture external destinations; local handled by bypass policy.
        if ctx.dst.starts_with("127.") || ctx.dst.starts_with("[::1]") {
            InterceptAction::Pass
        } else {
            InterceptAction::Capture
        }
    }
}

/// UDP interception policy.
#[derive(Debug, Default, Clone)]
pub struct UdpIntercept {
    pub capture_dns: bool,
}

impl UdpIntercept {
    pub fn inspect(&self, ctx: &InterceptContext) -> InterceptAction {
        if ctx.protocol != "udp" {
            return InterceptAction::Pass;
        }
        if self.capture_dns && (ctx.dst.ends_with(":53") || ctx.dst.ends_with(":853")) {
            return InterceptAction::Capture;
        }
        if ctx.dst.starts_with("127.") {
            InterceptAction::Pass
        } else {
            InterceptAction::Capture
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_captures_remote() {
        let t = TcpIntercept::default();
        assert_eq!(
            t.inspect(&InterceptContext {
                src: "10.0.0.2:1234".into(),
                dst: "1.1.1.1:443".into(),
                protocol: "tcp",
            }),
            InterceptAction::Capture
        );
    }

    #[test]
    fn udp_dns_optional() {
        let mut u = UdpIntercept { capture_dns: true };
        assert_eq!(
            u.inspect(&InterceptContext {
                src: "10.0.0.2:5555".into(),
                dst: "8.8.8.8:53".into(),
                protocol: "udp",
            }),
            InterceptAction::Capture
        );
        u.capture_dns = false;
        // still captures non-local by default
        assert_eq!(
            u.inspect(&InterceptContext {
                src: "10.0.0.2:5555".into(),
                dst: "8.8.8.8:53".into(),
                protocol: "udp",
            }),
            InterceptAction::Capture
        );
    }
}
