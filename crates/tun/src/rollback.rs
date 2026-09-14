//! Safe rollback and cleanup (NP-071).

use crate::device::{TunDevice, TunError, TunState};
use crate::packet::PacketPipeline;
use crate::route::RouteManager;

/// Snapshot of TUN-related state for transactional rollback.
#[derive(Debug, Default)]
pub struct TunSessionGuard {
    routes_installed: bool,
    device_started: bool,
}

impl TunSessionGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_routes(&mut self) {
        self.routes_installed = true;
    }

    pub fn mark_device_started(&mut self) {
        self.device_started = true;
    }

    /// Best-effort cleanup: stop device, clear routes, drain queues.
    pub fn rollback(
        &mut self,
        device: &mut TunDevice,
        routes: &mut RouteManager,
        ingress: &mut PacketPipeline,
        egress: &mut PacketPipeline,
    ) -> Result<(), TunError> {
        // Drain pipelines first so no stale packets remain.
        while ingress.pop().is_some() {}
        while egress.pop().is_some() {}

        if self.routes_installed || !routes.is_empty() {
            routes.remove_all()?;
            self.routes_installed = false;
        }

        if self.device_started || matches!(device.state(), TunState::Running | TunState::Starting) {
            let _ = device.stop();
            self.device_started = false;
        }

        Ok(())
    }
}

/// Full cleanup helper used on Core stop / failure paths.
pub fn cleanup_all(
    device: &mut TunDevice,
    routes: &mut RouteManager,
    ingress: &mut PacketPipeline,
    egress: &mut PacketPipeline,
) -> Result<(), TunError> {
    let mut guard = TunSessionGuard {
        routes_installed: true,
        device_started: true,
    };
    guard.rollback(device, routes, ingress, egress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::TunConfig;
    use crate::packet::{PacketDirection, PacketMeta, PacketPipeline};
    use crate::provider::{MockTunProvider, TunProvider};
    use crate::route::{RouteEntry, RouteManager};

    #[test]
    fn rollback_stops_and_clears() {
        let mut provider = MockTunProvider::new();
        let mut dev = provider.open(TunConfig::default()).unwrap();
        dev.configure_ip("10.0.0.1".into(), 24).unwrap();
        dev.start().unwrap();

        let mut routes = RouteManager::new();
        routes
            .install(RouteEntry {
                destination: "0.0.0.0/0".into(),
                gateway: Some("10.0.0.1".into()),
                metric: 1,
                via_tun: true,
            })
            .unwrap();

        let mut ingress = PacketPipeline::ingress();
        let mut egress = PacketPipeline::egress();
        ingress
            .push(PacketMeta::synthetic_ipv4(PacketDirection::Ingress, 20))
            .unwrap();

        let mut guard = TunSessionGuard::new();
        guard.mark_device_started();
        guard.mark_routes();
        guard
            .rollback(&mut dev, &mut routes, &mut ingress, &mut egress)
            .unwrap();

        assert_eq!(dev.state(), TunState::Stopped);
        assert!(routes.is_empty());
        assert_eq!(ingress.len(), 0);
    }
}
