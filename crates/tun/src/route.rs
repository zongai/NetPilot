//! Route installation / removal policy (NP-065).

use crate::device::TunError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub destination: String,
    pub gateway: Option<String>,
    pub metric: u32,
    pub via_tun: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteOp {
    Install,
    Remove,
}

/// Tracks desired routes; OS apply is deferred to a privileged helper.
#[derive(Debug, Default)]
pub struct RouteManager {
    entries: Vec<RouteEntry>,
}

impl RouteManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[RouteEntry] {
        &self.entries
    }

    pub fn install(&mut self, entry: RouteEntry) -> Result<(), TunError> {
        if entry.destination.is_empty() {
            return Err(TunError::InvalidConfig("empty destination"));
        }
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| e.destination == entry.destination)
        {
            self.entries[pos] = entry;
        } else {
            self.entries.push(entry);
        }
        Ok(())
    }

    pub fn remove(&mut self, destination: &str) -> Result<bool, TunError> {
        let before = self.entries.len();
        self.entries.retain(|e| e.destination != destination);
        Ok(self.entries.len() != before)
    }

    pub fn remove_all(&mut self) -> Result<(), TunError> {
        self.entries.clear();
        Ok(())
    }

    /// Default full-tunnel policy: default route via TUN with high metric optional.
    pub fn apply_full_tunnel(&mut self, tun_gateway: &str) -> Result<(), TunError> {
        self.install(RouteEntry {
            destination: "0.0.0.0/0".into(),
            gateway: Some(tun_gateway.into()),
            metric: 5,
            via_tun: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_replace_remove() {
        let mut m = RouteManager::new();
        m.install(RouteEntry {
            destination: "10.0.0.0/8".into(),
            gateway: None,
            metric: 1,
            via_tun: true,
        })
        .unwrap();
        m.install(RouteEntry {
            destination: "10.0.0.0/8".into(),
            gateway: Some("10.0.0.1".into()),
            metric: 2,
            via_tun: true,
        })
        .unwrap();
        assert_eq!(m.len(), 1);
        assert!(m.remove("10.0.0.0/8").unwrap());
        assert!(m.is_empty());
    }
}
