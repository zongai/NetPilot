//! Active proxy runtime (NP-173).

use crate::registry::ProxyRegistry;
use crate::{ProxyLifecycle, ProxyProfile};

#[derive(Debug)]
pub struct ActiveProxyRuntime {
    registry: ProxyRegistry,
    lifecycle: ProxyLifecycle,
}

impl ActiveProxyRuntime {
    pub fn new() -> Self {
        Self {
            registry: ProxyRegistry::new(),
            lifecycle: ProxyLifecycle::Created,
        }
    }

    pub fn registry(&self) -> &ProxyRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ProxyRegistry {
        &mut self.registry
    }

    pub fn lifecycle(&self) -> ProxyLifecycle {
        self.lifecycle
    }

    pub fn select(&mut self, id: &str) -> Result<&ProxyProfile, String> {
        self.registry.select_active(id)?;
        self.lifecycle = ProxyLifecycle::Validating;
        self.registry
            .active()
            .ok_or_else(|| "active missing".into())
    }

    pub fn mark_starting(&mut self) {
        self.lifecycle = ProxyLifecycle::Starting;
    }

    pub fn mark_running(&mut self) {
        self.lifecycle = ProxyLifecycle::Running;
    }

    pub fn mark_failed(&mut self) {
        self.lifecycle = ProxyLifecycle::Failed;
    }

    pub fn disconnect(&mut self) {
        self.lifecycle = ProxyLifecycle::Stopped;
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            active_id: self.registry.active_id().map(str::to_string),
            lifecycle: self.lifecycle,
            count: self.registry.len(),
        }
    }
}

impl Default for ActiveProxyRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub active_id: Option<String>,
    pub lifecycle: ProxyLifecycle,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtocolKind;

    #[test]
    fn select_connect_flow() {
        let mut rt = ActiveProxyRuntime::new();
        rt.registry_mut().upsert(ProxyProfile {
            id: "x".into(),
            name: "x".into(),
            protocol: ProtocolKind::Socks5,
            transport: None,
            server: "1.1.1.1".into(),
            port: 1080,
            password: None,
            uuid: None,
            username: None,
            sni: None,
            alpn: None,
            path: None,
            host: None,
            flow: None,
            network: None,
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        });
        rt.select("x").unwrap();
        rt.mark_starting();
        rt.mark_running();
        assert_eq!(rt.lifecycle(), ProxyLifecycle::Running);
        assert_eq!(rt.snapshot().active_id.as_deref(), Some("x"));
    }
}
