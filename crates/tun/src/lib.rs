//! TUN provider abstraction and packet pipelines (NP-061…NP-072).
//!
//! Windows Wintun integration is intentionally behind traits; unit tests use
//! an in-memory mock. No `unsafe` in this crate yet.

#![cfg_attr(all(not(feature = "wintun-native"), not(windows)), forbid(unsafe_code))]
#![cfg_attr(any(feature = "wintun-native", windows), allow(unsafe_code))]

mod bypass;
mod device;
mod e2e;
mod intercept;
mod packet;
mod provider;
mod rollback;
mod route;
mod wintun;
mod wintun_spike;

pub use bypass::{BypassDecision, BypassPolicy, BypassReason};
pub use device::{TunConfig, TunDevice, TunError, TunState};
pub use e2e::{run_mock_e2e, E2eReport};
pub use intercept::{InterceptAction, InterceptContext, TcpIntercept, UdpIntercept};
pub use packet::{PacketBatch, PacketDirection, PacketMeta, PacketPipeline};
pub use provider::{MockTunProvider, TunProvider};
pub use rollback::{cleanup_all, TunSessionGuard};
pub use route::{RouteEntry, RouteManager, RouteOp};
pub use wintun::{
    WintunAdapterRequest, WintunDllPath, WintunSession, WintunSessionState, WintunTunProvider,
};
pub use wintun_spike::{WintunFeasibility, WintunStatus};

pub const CRATE_NAME: &str = "netpilot-tun";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_lifecycle_and_io() {
        let mut provider = MockTunProvider::new();
        let mut dev = provider.open(TunConfig::default()).unwrap();
        assert_eq!(dev.state(), TunState::Created);
        dev.configure_ip("10.0.0.1".into(), 24).unwrap();
        dev.start().unwrap();
        assert_eq!(dev.state(), TunState::Running);

        let mut ingress = PacketPipeline::ingress();
        let mut egress = PacketPipeline::egress();
        let pkt = PacketMeta::synthetic_ipv4(PacketDirection::Ingress, 20);
        ingress.push(pkt.clone()).unwrap();
        assert_eq!(ingress.pop().unwrap().direction, PacketDirection::Ingress);
        egress
            .push(PacketMeta::synthetic_ipv4(PacketDirection::Egress, 20))
            .unwrap();

        let mut routes = RouteManager::new();
        routes
            .install(RouteEntry {
                destination: "0.0.0.0/0".into(),
                gateway: Some("10.0.0.1".into()),
                metric: 1,
                via_tun: true,
            })
            .unwrap();
        assert_eq!(routes.len(), 1);
        routes.remove_all().unwrap();

        let bypass = BypassPolicy::default();
        assert_eq!(
            bypass.decide("127.0.0.1", 80),
            BypassDecision::Bypass(BypassReason::Loopback)
        );

        let tcp = TcpIntercept::default();
        assert_eq!(
            tcp.inspect(&InterceptContext {
                src: "1.2.3.4:1234".into(),
                dst: "8.8.8.8:443".into(),
                protocol: "tcp",
            }),
            InterceptAction::Capture
        );

        dev.stop().unwrap();
        assert_eq!(dev.state(), TunState::Stopped);
    }

    #[test]
    fn wintun_spike_reports_status() {
        let f = WintunFeasibility::evaluate();
        assert!(!f.notes.is_empty());
        assert!(matches!(
            f.status,
            WintunStatus::Deferred | WintunStatus::Available | WintunStatus::Unavailable
        ));
    }
}
