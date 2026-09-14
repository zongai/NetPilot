//! Windows TUN E2E harness (NP-072).
//!
//! Real adapter tests are `#[cfg(all(windows, feature = "wintun-e2e"))]` later;
//! default harness exercises the mock path deterministically on all hosts.

use crate::bypass::BypassPolicy;
use crate::device::{TunConfig, TunError, TunState};
use crate::intercept::{InterceptContext, TcpIntercept, UdpIntercept};
use crate::packet::{PacketDirection, PacketMeta, PacketPipeline};
use crate::provider::{MockTunProvider, TunProvider};
use crate::rollback::cleanup_all;
use crate::route::{RouteEntry, RouteManager};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E2eReport {
    pub steps_ok: Vec<&'static str>,
    pub failed_step: Option<&'static str>,
    pub error: Option<String>,
}

impl E2eReport {
    pub fn success(steps: Vec<&'static str>) -> Self {
        Self {
            steps_ok: steps,
            failed_step: None,
            error: None,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.failed_step.is_none()
    }
}

/// Run a deterministic mock E2E sequence approximating the Windows path.
pub fn run_mock_e2e() -> E2eReport {
    let mut steps = Vec::new();
    let result = (|| -> Result<(), TunError> {
        let mut provider = MockTunProvider::new();
        steps.push("open_provider");
        let mut dev = provider.open(TunConfig {
            name: "NetPilot-E2E".into(),
            mtu: 1500,
            ipv4: None,
            prefix: 24,
        })?;
        steps.push("open_device");

        dev.configure_ip("10.255.0.1".into(), 24)?;
        steps.push("configure_ip");

        dev.start()?;
        if dev.state() != TunState::Running {
            return Err(TunError::FailedPrecondition("not running"));
        }
        steps.push("start");

        let mut routes = RouteManager::new();
        routes.install(RouteEntry {
            destination: "0.0.0.0/0".into(),
            gateway: Some("10.255.0.1".into()),
            metric: 5,
            via_tun: true,
        })?;
        steps.push("install_routes");

        let mut ingress = PacketPipeline::ingress();
        let mut egress = PacketPipeline::egress();
        ingress.push(PacketMeta::synthetic_ipv4(PacketDirection::Ingress, 40))?;
        egress.push(PacketMeta::synthetic_ipv4(PacketDirection::Egress, 40))?;
        steps.push("packet_io");

        let bypass = BypassPolicy::default();
        let _ = bypass.decide("127.0.0.1", 80);
        let tcp = TcpIntercept::default();
        let _ = tcp.inspect(&InterceptContext {
            src: "10.255.0.2:12345".into(),
            dst: "1.1.1.1:443".into(),
            protocol: "tcp",
        });
        let udp = UdpIntercept {
            capture_dns: true,
        };
        let _ = udp.inspect(&InterceptContext {
            src: "10.255.0.2:53000".into(),
            dst: "8.8.8.8:53".into(),
            protocol: "udp",
        });
        steps.push("intercept_bypass");

        cleanup_all(&mut dev, &mut routes, &mut ingress, &mut egress)?;
        steps.push("cleanup");
        Ok(())
    })();

    match result {
        Ok(()) => E2eReport::success(steps),
        Err(e) => E2eReport {
            steps_ok: steps,
            failed_step: Some("see error"),
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_e2e_passes() {
        let r = run_mock_e2e();
        assert!(r.is_ok(), "{r:?}");
        assert!(r.steps_ok.contains(&"cleanup"));
    }
}
