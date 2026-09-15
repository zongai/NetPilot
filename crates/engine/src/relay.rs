//! TUN ↔ outbound relay: map netstack flows to dialed OutboundStreams.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;

use netpilot_netstack::{FourTuple, StackEventKind};
use netpilot_outbound::{dial_outbound, DialRequest, OutboundStream};
use netpilot_proxy::ProxyProfile;

#[derive(Debug, Default)]
pub struct RelayStats {
    pub flows: u64,
    pub dial_ok: u64,
    pub dial_fail: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub pumps: u64,
}

struct RelayFlow {
    stream: OutboundStream,
    outbound: String,
}

#[derive(Default)]
pub struct TunRelay {
    flows: HashMap<FourTuple, RelayFlow>,
    pub stats: RelayStats,
}

impl TunRelay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn flow_count(&self) -> usize {
        self.flows.len()
    }

    pub fn open_flow(
        &mut self,
        tuple: FourTuple,
        outbound: &str,
        profiles: &[ProxyProfile],
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<(), String> {
        if self.flows.contains_key(&tuple) {
            return Ok(());
        }
        self.stats.flows = self.stats.flows.saturating_add(1);
        let mut req = DialRequest::new(host, port);
        req.timeout = timeout;
        match dial_outbound(outbound, profiles, &req) {
            Ok((mut stream, _)) => {
                // Prefer non-blocking reads for pump loop fairness.
                if let OutboundStream::Plain(ref s) = stream {
                    let _ = s.set_nonblocking(true);
                    let _ = s.set_read_timeout(Some(Duration::from_millis(1)));
                }
                let _ = stream.set_read_timeout(Some(Duration::from_millis(5)));
                self.flows.insert(
                    tuple,
                    RelayFlow {
                        stream,
                        outbound: outbound.to_string(),
                    },
                );
                self.stats.dial_ok = self.stats.dial_ok.saturating_add(1);
                Ok(())
            }
            Err(e) => {
                self.stats.dial_fail = self.stats.dial_fail.saturating_add(1);
                Err(e.to_string())
            }
        }
    }

    pub fn write_up(&mut self, tuple: &FourTuple, data: &[u8]) -> Result<usize, String> {
        let flow = self
            .flows
            .get_mut(tuple)
            .ok_or_else(|| "no flow".to_string())?;
        flow.stream.write_all(data).map_err(|e| e.to_string())?;
        let _ = flow.stream.flush();
        self.stats.bytes_up = self.stats.bytes_up.saturating_add(data.len() as u64);
        Ok(data.len())
    }

    /// Read available data from all flows; returns (tuple, bytes) list.
    pub fn poll_down(&mut self) -> Vec<(FourTuple, Vec<u8>)> {
        let mut out = Vec::new();
        let keys: Vec<FourTuple> = self.flows.keys().cloned().collect();
        for key in keys {
            let Some(flow) = self.flows.get_mut(&key) else {
                continue;
            };
            let mut buf = [0u8; 16 * 1024];
            match flow.stream.read(&mut buf) {
                Ok(0) => {
                    // EOF — drop flow
                    self.flows.remove(&key);
                }
                Ok(n) => {
                    self.stats.bytes_down = self.stats.bytes_down.saturating_add(n as u64);
                    out.push((key, buf[..n].to_vec()));
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => {
                    self.flows.remove(&key);
                }
            }
        }
        out
    }

    pub fn close_flow(&mut self, tuple: &FourTuple) {
        self.flows.remove(tuple);
    }

    pub fn clear(&mut self) {
        self.flows.clear();
    }
}

/// Destination host for dialing: prefer domain from fake-ip map, else dotted IP.
pub fn dial_target_from_event(dst: &str, dport: u16) -> (String, u16) {
    (dst.to_string(), dport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_relay() {
        let r = TunRelay::new();
        assert_eq!(r.flow_count(), 0);
    }
}
