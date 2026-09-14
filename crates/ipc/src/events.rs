//! Server event stream (NP-019).

use std::collections::VecDeque;

use crate::{IpcEnvelope, MessageKind};

/// Bounded in-memory event bus for Core → client notifications.
#[derive(Debug)]
pub struct EventStream {
    capacity: usize,
    queue: VecDeque<IpcEnvelope>,
    drop_count: u64,
}

impl EventStream {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            queue: VecDeque::new(),
            drop_count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dropped(&self) -> u64 {
        self.drop_count
    }

    /// Publish an event envelope; drops oldest when full.
    pub fn publish(&mut self, event: IpcEnvelope) -> Result<(), EventError> {
        if event.kind != MessageKind::Event {
            return Err(EventError::InvalidInput("expected event kind"));
        }
        event
            .validate_version()
            .map_err(|e| EventError::Envelope(e.to_string()))?;
        if self.queue.len() >= self.capacity {
            self.queue.pop_front();
            self.drop_count = self.drop_count.saturating_add(1);
        }
        self.queue.push_back(event);
        Ok(())
    }

    /// Helper: runtime state change event.
    pub fn publish_state(&mut self, request_id: &str, state: &str) -> Result<(), EventError> {
        let env = IpcEnvelope::event(request_id, "runtime.state_changed")
            .with_payload(serde_json::json!({ "state": state }));
        self.publish(env)
    }

    pub fn poll(&mut self) -> Option<IpcEnvelope> {
        self.queue.pop_front()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new(64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventError {
    InvalidInput(&'static str),
    Envelope(String),
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(m) => write!(f, "InvalidInput: {m}"),
            Self::Envelope(m) => write!(f, "Envelope: {m}"),
        }
    }
}

impl std::error::Error for EventError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_and_poll() {
        let mut stream = EventStream::new(4);
        stream.publish_state("r1", "running").unwrap();
        let ev = stream.poll().unwrap();
        assert_eq!(ev.kind, MessageKind::Event);
        assert_eq!(ev.operation.as_deref(), Some("runtime.state_changed"));
    }

    #[test]
    fn drops_when_full() {
        let mut stream = EventStream::new(2);
        stream.publish_state("1", "a").unwrap();
        stream.publish_state("2", "b").unwrap();
        stream.publish_state("3", "c").unwrap();
        assert_eq!(stream.dropped(), 1);
        assert_eq!(stream.len(), 2);
        assert_eq!(stream.poll().unwrap().request_id, "2");
    }

    #[test]
    fn reject_non_event() {
        let mut stream = EventStream::new(4);
        let req = IpcEnvelope::request("x", "y");
        assert!(matches!(
            stream.publish(req),
            Err(EventError::InvalidInput(_))
        ));
    }
}
