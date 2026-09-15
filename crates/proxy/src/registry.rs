//! Proxy registry (NP-171).

use crate::{ProxyGroup, ProxyProfile};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct ProxyRegistry {
    profiles: HashMap<String, ProxyProfile>,
    groups: HashMap<String, ProxyGroup>,
    active_id: Option<String>,
}

impl ProxyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, profile: ProxyProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    pub fn remove(&mut self, id: &str) -> Option<ProxyProfile> {
        if self.active_id.as_deref() == Some(id) {
            self.active_id = None;
        }
        self.profiles.remove(id)
    }

    pub fn get(&self, id: &str) -> Option<&ProxyProfile> {
        self.profiles.get(id)
    }

    pub fn list(&self) -> Vec<&ProxyProfile> {
        let mut v: Vec<_> = self.profiles.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn upsert_group(&mut self, group: ProxyGroup) {
        self.groups.insert(group.id.clone(), group);
    }

    pub fn groups(&self) -> Vec<&ProxyGroup> {
        self.groups.values().collect()
    }

    pub fn select_active(&mut self, id: &str) -> Result<(), String> {
        if !self.profiles.contains_key(id) {
            return Err(format!("unknown proxy id: {id}"));
        }
        self.active_id = Some(id.to_string());
        Ok(())
    }

    pub fn active(&self) -> Option<&ProxyProfile> {
        self.active_id.as_ref().and_then(|id| self.profiles.get(id))
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtocolKind;

    fn sample(id: &str) -> ProxyProfile {
        ProxyProfile {
            id: id.into(),
            name: id.into(),
            protocol: ProtocolKind::Socks5,
            transport: None,
            server: "127.0.0.1".into(),
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
        }
    }

    #[test]
    fn upsert_select() {
        let mut r = ProxyRegistry::new();
        r.upsert(sample("a"));
        r.select_active("a").unwrap();
        assert_eq!(r.active().unwrap().id, "a");
    }
}
