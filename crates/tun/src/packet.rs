//! Packet ingress / egress pipelines (NP-066 / NP-067).

use crate::device::TunError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketDirection {
    Ingress,
    Egress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketMeta {
    pub direction: PacketDirection,
    pub len: usize,
    pub ipv4: bool,
    /// First bytes preview for tests (not a full frame buffer).
    pub preview: Vec<u8>,
}

impl PacketMeta {
    pub fn synthetic_ipv4(direction: PacketDirection, len: usize) -> Self {
        Self {
            direction,
            len,
            ipv4: true,
            preview: vec![0x45, 0x00],
        }
    }
}

#[derive(Debug)]
pub struct PacketBatch {
    pub packets: Vec<PacketMeta>,
}

/// Bounded queue for one direction.
#[derive(Debug)]
pub struct PacketPipeline {
    direction: PacketDirection,
    queue: std::collections::VecDeque<PacketMeta>,
    capacity: usize,
    dropped: u64,
}

impl PacketPipeline {
    pub fn ingress() -> Self {
        Self::new(PacketDirection::Ingress, 256)
    }

    pub fn egress() -> Self {
        Self::new(PacketDirection::Egress, 256)
    }

    pub fn new(direction: PacketDirection, capacity: usize) -> Self {
        Self {
            direction,
            queue: std::collections::VecDeque::new(),
            capacity: capacity.max(1),
            dropped: 0,
        }
    }

    pub fn direction(&self) -> PacketDirection {
        self.direction
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn push(&mut self, packet: PacketMeta) -> Result<(), TunError> {
        if packet.direction != self.direction {
            return Err(TunError::InvalidConfig("packet direction mismatch"));
        }
        if self.queue.len() >= self.capacity {
            self.queue.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.queue.push_back(packet);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<PacketMeta> {
        self.queue.pop_front()
    }

    pub fn drain(&mut self, max: usize) -> PacketBatch {
        let mut packets = Vec::new();
        while packets.len() < max {
            match self.queue.pop_front() {
                Some(p) => packets.push(p),
                None => break,
            }
        }
        PacketBatch { packets }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_oldest_when_full() {
        let mut p = PacketPipeline::new(PacketDirection::Ingress, 2);
        p.push(PacketMeta::synthetic_ipv4(PacketDirection::Ingress, 1))
            .unwrap();
        p.push(PacketMeta::synthetic_ipv4(PacketDirection::Ingress, 2))
            .unwrap();
        p.push(PacketMeta::synthetic_ipv4(PacketDirection::Ingress, 3))
            .unwrap();
        assert_eq!(p.dropped(), 1);
        assert_eq!(p.len(), 2);
    }
}
