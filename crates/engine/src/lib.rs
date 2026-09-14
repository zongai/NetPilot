//! Traffic engine: load profiles + rules, decide route, dial outbound, manage TUN.

#![forbid(unsafe_code)]

mod inbound;

use std::time::Duration;

use netpilot_dns::{DnsRoutePolicy, FakeIpAllocator};
use netpilot_outbound::{dial_outbound, DialReport, DialRequest, OutboundError, OutboundStream};
use netpilot_proxy::ProxyProfile;
use netpilot_routing::{parse_rules, RouteRequest, RoutingEngine, RuleIndex};
use netpilot_tun::{TunConfig, TunError, WintunSession, WintunSessionState, WintunTunProvider};

pub use inbound::{start_socks_inbound, InboundStats, SocksInbound};

pub const CRATE_NAME: &str = "netpilot-engine";

#[derive(Debug)]
pub enum EngineError {
    Rules(String),
    Outbound(OutboundError),
    Tun(String),
    State(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rules(m) => write!(f, "Rules({m})"),
            Self::Outbound(e) => write!(f, "Outbound({e})"),
            Self::Tun(m) => write!(f, "Tun({m})"),
            Self::State(m) => write!(f, "State({m})"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<OutboundError> for EngineError {
    fn from(e: OutboundError) -> Self {
        Self::Outbound(e)
    }
}

impl From<TunError> for EngineError {
    fn from(e: TunError) -> Self {
        Self::Tun(e.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    pub outbound: String,
    pub explanation: String,
    pub matcher: String,
}

#[derive(Debug, Clone)]
pub struct TunnelStatus {
    pub running: bool,
    pub native: bool,
    pub adapter: String,
    pub state: String,
    pub packets_in: u64,
    pub packets_out: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineStats {
    pub routed: u64,
    pub direct: u64,
    pub proxy: u64,
    pub reject: u64,
    pub dial_fail: u64,
}

/// Central runtime for data-plane control.
pub struct TrafficEngine {
    profiles: Vec<ProxyProfile>,
    selected_outbound: Option<String>,
    rules: Option<RoutingEngine>,
    tun_provider: WintunTunProvider,
    tun_session: Option<WintunSession>,
    fake_ip: FakeIpAllocator,
    #[allow(dead_code)]
    dns_policy: DnsRoutePolicy,
    dial_timeout: Duration,
    stats: EngineStats,
    inbound: Option<SocksInbound>,
}

impl Default for TrafficEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficEngine {
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
            selected_outbound: None,
            rules: None,
            tun_provider: WintunTunProvider::new(),
            tun_session: None,
            fake_ip: FakeIpAllocator::new("198.18.0.0", 16),
            dns_policy: DnsRoutePolicy::default(),
            dial_timeout: Duration::from_secs(10),
            stats: EngineStats::default(),
            inbound: None,
        }
    }

    pub fn set_profiles(&mut self, profiles: Vec<ProxyProfile>) {
        self.profiles = profiles;
    }

    pub fn profiles(&self) -> &[ProxyProfile] {
        &self.profiles
    }

    pub fn add_profile(&mut self, profile: ProxyProfile) {
        if let Some(i) = self.profiles.iter().position(|p| p.id == profile.id) {
            self.profiles[i] = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    pub fn select_outbound(&mut self, name: impl Into<String>) {
        self.selected_outbound = Some(name.into());
    }

    pub fn selected_outbound(&self) -> Option<&str> {
        self.selected_outbound.as_deref()
    }

    pub fn load_rules_text(&mut self, text: &str) -> Result<usize, EngineError> {
        let rules = parse_rules(text).map_err(|e| EngineError::Rules(e.to_string()))?;
        let n = rules.len();
        self.rules = Some(RoutingEngine::new(RuleIndex::new(rules)));
        Ok(n)
    }

    pub fn decide(&self, host: Option<&str>, ip: Option<&str>, port: Option<u16>) -> RouteResult {
        let mut rreq = RouteRequest::default();
        if let Some(h) = host {
            rreq = RouteRequest::domain(h);
        }
        if let Some(i) = ip {
            rreq.ip = Some(i.to_string());
        }
        if let Some(p) = port {
            rreq.port = Some(p);
        }
        if let Some(eng) = self.rules.as_ref() {
            let (d, e) = eng.decide(&rreq);
            RouteResult {
                outbound: d.outbound,
                explanation: e.summary(),
                matcher: e.matcher_kind.clone(),
            }
        } else if let Some(sel) = &self.selected_outbound {
            RouteResult {
                outbound: sel.clone(),
                explanation: "manual selection".into(),
                matcher: "manual".into(),
            }
        } else {
            RouteResult {
                outbound: "DIRECT".into(),
                explanation: "no rules; default DIRECT".into(),
                matcher: "default".into(),
            }
        }
    }

    /// Route + dial: opens then drops the stream (connectivity probe path).
    pub fn route_and_dial(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<(RouteResult, DialReport), EngineError> {
        let route = self.decide(Some(host), None, Some(port));
        self.stats.routed = self.stats.routed.saturating_add(1);
        match route.outbound.to_ascii_uppercase().as_str() {
            "DIRECT" => self.stats.direct = self.stats.direct.saturating_add(1),
            "REJECT" | "BLOCK" => {
                self.stats.reject = self.stats.reject.saturating_add(1);
                return Err(OutboundError::Rejected.into());
            }
            _ => self.stats.proxy = self.stats.proxy.saturating_add(1),
        }
        let mut req = DialRequest::new(host, port);
        req.timeout = self.dial_timeout;
        match dial_outbound(&route.outbound, &self.profiles, &req) {
            Ok((_stream, report)) => Ok((route, report)),
            Err(e) => {
                self.stats.dial_fail = self.stats.dial_fail.saturating_add(1);
                Err(e.into())
            }
        }
    }

    /// Open a live outbound stream after rule decision.
    pub fn open_stream(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<(RouteResult, DialReport, OutboundStream), EngineError> {
        let route = self.decide(Some(host), None, Some(port));
        self.stats.routed = self.stats.routed.saturating_add(1);
        if route.outbound.eq_ignore_ascii_case("REJECT") {
            self.stats.reject = self.stats.reject.saturating_add(1);
            return Err(OutboundError::Rejected.into());
        }
        if route.outbound.eq_ignore_ascii_case("DIRECT") {
            self.stats.direct = self.stats.direct.saturating_add(1);
        } else {
            self.stats.proxy = self.stats.proxy.saturating_add(1);
        }
        let mut req = DialRequest::new(host, port);
        req.timeout = self.dial_timeout;
        match dial_outbound(&route.outbound, &self.profiles, &req) {
            Ok((stream, report)) => Ok((route, report, stream)),
            Err(e) => {
                self.stats.dial_fail = self.stats.dial_fail.saturating_add(1);
                Err(e.into())
            }
        }
    }

    pub fn start_tunnel(&mut self, name: Option<&str>) -> Result<TunnelStatus, EngineError> {
        if self.tun_session.is_some() {
            return Ok(self.tunnel_status());
        }
        let mut config = TunConfig::default();
        if let Some(n) = name {
            config.name = n.to_string();
        }
        config.ipv4 = Some("10.0.0.1".into());
        config.prefix = 24;
        let session = self.tun_provider.open_session(config)?;
        self.tun_session = Some(session);
        Ok(self.tunnel_status())
    }

    pub fn stop_tunnel(&mut self) -> Result<TunnelStatus, EngineError> {
        if let Some(mut s) = self.tun_session.take() {
            let _ = s.stop();
        }
        Ok(self.tunnel_status())
    }

    pub fn tunnel_status(&self) -> TunnelStatus {
        match &self.tun_session {
            Some(s) => TunnelStatus {
                running: s.state() == WintunSessionState::SessionRunning,
                native: s.is_native(),
                adapter: s.adapter_name().to_string(),
                state: format!("{:?}", s.state()),
                packets_in: s.stats().0,
                packets_out: s.stats().1,
                last_error: s.last_error().map(|x| x.to_string()),
            },
            None => TunnelStatus {
                running: false,
                native: false,
                adapter: String::new(),
                state: "stopped".into(),
                packets_in: 0,
                packets_out: 0,
                last_error: None,
            },
        }
    }

    pub fn pump_tun_once(&mut self, timeout_ms: u32) -> Result<Option<usize>, EngineError> {
        let session = self
            .tun_session
            .as_mut()
            .ok_or_else(|| EngineError::State("tunnel not running".into()))?;
        match session.receive_packet(timeout_ms)? {
            Some(pkt) => Ok(Some(pkt.len())),
            None => Ok(None),
        }
    }

    pub fn allocate_fake_ip(&mut self, host: &str) -> String {
        self.fake_ip
            .allocate(host)
            .unwrap_or_else(|_| "198.18.0.1".into())
    }


    pub fn attach_inbound(&mut self, inbound: SocksInbound) {
        self.inbound = Some(inbound);
    }

    pub fn stop_attached_inbound(&mut self) {
        if let Some(ib) = self.inbound.take() {
            ib.stop();
        }
    }

    pub fn inbound_port(&self) -> Option<u16> {
        self.inbound.as_ref().map(|i| i.port())
    }

    pub fn stats(&self) -> &EngineStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use netpilot_proxy::{ProtocolKind, ProxyProfile};

    #[test]
    fn decide_default_direct() {
        let eng = TrafficEngine::new();
        let r = eng.decide(Some("example.com"), None, Some(443));
        assert_eq!(r.outbound, "DIRECT");
    }

    #[test]
    fn load_rules_and_decide() {
        let mut eng = TrafficEngine::new();
        eng.load_rules_text("DOMAIN-SUFFIX,google.com,PROXY\nMATCH,DIRECT\n")
            .unwrap();
        let r = eng.decide(Some("www.google.com"), None, None);
        assert_eq!(r.outbound, "PROXY");
    }

    #[test]
    fn tunnel_logical_start_stop() {
        let mut eng = TrafficEngine::new();
        let st = eng.start_tunnel(Some("NetPilotTest")).unwrap();
        assert!(st.running);
        let st = eng.stop_tunnel().unwrap();
        assert!(!st.running);
    }

    #[test]
    fn profile_select() {
        let mut eng = TrafficEngine::new();
        eng.add_profile(ProxyProfile {
            id: "p1".into(),
            name: "node1".into(),
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
            tags: vec![],
        });
        eng.select_outbound("p1");
        assert_eq!(eng.selected_outbound(), Some("p1"));
    }
}

/// Minimal IPv4 header peek for TUN packets (no full reassembly).
#[derive(Debug, Clone)]
pub struct Ipv4Peek {
    pub src: String,
    pub dst: String,
    pub protocol: u8,
    pub dst_port: Option<u16>,
    pub src_port: Option<u16>,
}

pub fn peek_ipv4(packet: &[u8]) -> Option<Ipv4Peek> {
    if packet.len() < 20 {
        return None;
    }
    if packet[0] >> 4 != 4 {
        return None;
    }
    let ihl = (packet[0] & 0x0f) as usize * 4;
    if packet.len() < ihl {
        return None;
    }
    let proto = packet[9];
    let src = format!(
        "{}.{}.{}.{}",
        packet[12], packet[13], packet[14], packet[15]
    );
    let dst = format!(
        "{}.{}.{}.{}",
        packet[16], packet[17], packet[18], packet[19]
    );
    let mut dst_port = None;
    let mut src_port = None;
    if (proto == 6 || proto == 17) && packet.len() >= ihl + 4 {
        src_port = Some(u16::from_be_bytes([packet[ihl], packet[ihl + 1]]));
        dst_port = Some(u16::from_be_bytes([packet[ihl + 2], packet[ihl + 3]]));
    }
    Some(Ipv4Peek {
        src,
        dst,
        protocol: proto,
        dst_port,
        src_port,
    })
}
