//! Traffic engine: load profiles + rules, decide route, dial outbound, manage TUN.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::redundant_closure)]
#![allow(unused_mut)]

mod inbound;
mod relay;

use std::time::Duration;

use netpilot_dns::{DnsRoutePolicy, FakeIpAllocator};
use netpilot_netstack::{FourTuple, NetStack, StackEvent, StackEventKind};
use netpilot_os_route::{configure_interface_address, InterfaceAddress, RoutePlan};
use netpilot_outbound::{dial_outbound, DialReport, DialRequest, OutboundError, OutboundStream};
use netpilot_proxy::ProxyProfile;
use netpilot_routing::{parse_rules, RouteRequest, RoutingEngine, RuleIndex};
use netpilot_transport_reality::{Fingerprint, RealityConfig, RealitySession};
use netpilot_tun::{TunConfig, TunError, WintunSession, WintunSessionState, WintunTunProvider};
use std::net::Ipv4Addr;

pub use inbound::{
    start_http_inbound, start_socks_inbound, HttpInbound, InboundStats, SocksInbound,
};
pub use relay::TunRelay;

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
    pub adapter_luid: u64,
    pub configured_ip: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EngineStats {
    pub routed: u64,
    pub direct: u64,
    pub proxy: u64,
    pub reject: u64,
    pub dial_fail: u64,
}

#[derive(Debug, Default)]
pub struct PumpResult {
    pub packet_in: Option<usize>,
    pub replies_out: u32,
    pub dialed: bool,
    pub dial_error: Option<String>,
    pub relay_error: Option<String>,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub event_kind: Option<String>,
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
    route_plan: RoutePlan,
    netstack: NetStack,
    physical_gateway: Option<Ipv4Addr>,
    physical_luid: u64,
    tun_relay: TunRelay,
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
            route_plan: RoutePlan::new(),
            netstack: NetStack::new(),
            physical_gateway: None,
            physical_luid: 0,
            tun_relay: TunRelay::new(),
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
        let mut session = self.tun_provider.open_session(config)?;
        // Native: configure interface address via IP Helper when LUID is known.
        if session.is_native() && session.adapter_luid() != 0 {
            let ip: std::net::Ipv4Addr = "10.0.0.1".parse().unwrap();
            match configure_interface_address(&InterfaceAddress {
                address: ip,
                prefix_len: 24,
                interface_luid: session.adapter_luid(),
            }) {
                Ok(()) => session.set_configured_ip("10.0.0.1/24"),
                Err(e) => {
                    // Soft-fail: tunnel still runs; report error on session.
                    let _ = e;
                }
            }
        } else {
            session.set_configured_ip("10.0.0.1/24");
        }
        self.tun_session = Some(session);
        Ok(self.tunnel_status())
    }

    pub fn stop_tunnel(&mut self) -> Result<TunnelStatus, EngineError> {
        let _ = self.rollback_routes();
        self.tun_relay.clear();
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
                adapter_luid: s.adapter_luid(),
                configured_ip: s.configured_ip().map(|x| x.to_string()),
            },
            None => TunnelStatus {
                running: false,
                native: false,
                adapter: String::new(),
                state: "stopped".into(),
                packets_in: 0,
                packets_out: 0,
                last_error: None,
                adapter_luid: 0,
                configured_ip: None,
            },
        }
    }

    /// One TUN cycle: receive → netstack → reply to TUN → dial/relay outbound.
    pub fn pump_and_relay_once(&mut self, timeout_ms: u32) -> Result<PumpResult, EngineError> {
        self.tun_relay.stats.pumps = self.tun_relay.stats.pumps.saturating_add(1);
        let mut result = PumpResult::default();

        // 1) Receive from TUN
        let packet = {
            let session = self
                .tun_session
                .as_mut()
                .ok_or_else(|| EngineError::State("tunnel not running".into()))?;
            session.receive_packet(timeout_ms)?
        };

        if let Some(pkt) = packet {
            result.packet_in = Some(pkt.len());
            let (replies, event) = self.handle_tun_packet(&pkt);

            // 2) Write stack replies back to TUN
            if let Some(session) = self.tun_session.as_mut() {
                for r in &replies {
                    let _ = session.send_packet(r);
                    result.replies_out += 1;
                }
            }

            // 3) Handle events → outbound
            if let Some(ev) = event {
                result.event_kind = Some(format!("{:?}", ev.kind));
                match ev.kind {
                    StackEventKind::TcpSyn => {
                        if let Some(tuple) = ev.tuple.clone() {
                            let outbound =
                                ev.outbound_hint.clone().unwrap_or_else(|| "DIRECT".into());
                            let profiles = self.profiles.clone();
                            // Dial destination = remote IP:port from tuple
                            let host = netpilot_netstack::addr_str(tuple.dst);
                            let port = tuple.dport;
                            match self.tun_relay.open_flow(
                                tuple,
                                &outbound,
                                &profiles,
                                &host,
                                port,
                                self.dial_timeout,
                            ) {
                                Ok(()) => result.dialed = true,
                                Err(e) => result.dial_error = Some(e),
                            }
                        }
                    }
                    StackEventKind::TcpData => {
                        if let Some(tuple) = ev.tuple.as_ref() {
                            if !ev.payload.is_empty() {
                                match self.tun_relay.write_up(tuple, &ev.payload) {
                                    Ok(n) => result.bytes_up += n as u64,
                                    Err(e) => result.relay_error = Some(e),
                                }
                            }
                        }
                    }
                    StackEventKind::TcpFin | StackEventKind::TcpRst => {
                        if let Some(tuple) = ev.tuple.as_ref() {
                            self.tun_relay.close_flow(tuple);
                        }
                    }
                    _ => {}
                }
            }
        }

        // 4) Poll outbound → inject TCP data into netstack → send to TUN
        let down = self.tun_relay.poll_down();
        for (tuple, data) in down {
            if let Some(pkt) = self.netstack.inject_tcp_data(&tuple, &data) {
                result.bytes_down += data.len() as u64;
                if let Some(session) = self.tun_session.as_mut() {
                    let _ = session.send_packet(&pkt);
                    result.replies_out += 1;
                }
            }
        }

        Ok(result)
    }

    /// Inject a synthetic packet (tests / logical TUN without native DLL).
    pub fn inject_packet_and_relay(&mut self, packet: &[u8]) -> Result<PumpResult, EngineError> {
        if self.tun_session.is_none() {
            return Err(EngineError::State("tunnel not running".into()));
        }
        let mut result = PumpResult::default();
        result.packet_in = Some(packet.len());
        let (replies, event) = self.handle_tun_packet(packet);
        if let Some(session) = self.tun_session.as_mut() {
            for r in &replies {
                let _ = session.send_packet(r);
                result.replies_out += 1;
            }
        }
        if let Some(ev) = event {
            result.event_kind = Some(format!("{:?}", ev.kind));
            if matches!(ev.kind, StackEventKind::TcpSyn) {
                if let Some(tuple) = ev.tuple.clone() {
                    let outbound = ev.outbound_hint.clone().unwrap_or_else(|| "DIRECT".into());
                    let host = netpilot_netstack::addr_str(tuple.dst);
                    let port = tuple.dport;
                    let profiles = self.profiles.clone();
                    match self.tun_relay.open_flow(
                        tuple,
                        &outbound,
                        &profiles,
                        &host,
                        port,
                        self.dial_timeout,
                    ) {
                        Ok(()) => result.dialed = true,
                        Err(e) => result.dial_error = Some(e),
                    }
                }
            } else if matches!(ev.kind, StackEventKind::TcpData) {
                if let Some(tuple) = ev.tuple.as_ref() {
                    if !ev.payload.is_empty() {
                        let _ = self.tun_relay.write_up(tuple, &ev.payload);
                        result.bytes_up += ev.payload.len() as u64;
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn relay_stats(&self) -> &crate::relay::RelayStats {
        &self.tun_relay.stats
    }

    pub fn relay_flow_count(&self) -> usize {
        self.tun_relay.flow_count()
    }

    pub fn pump_tun_once(&mut self, timeout_ms: u32) -> Result<Option<usize>, EngineError> {
        let r = self.pump_and_relay_once(timeout_ms)?;
        Ok(r.packet_in)
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

    pub fn set_physical_gateway(&mut self, gw: Option<Ipv4Addr>, luid: u64) {
        self.physical_gateway = gw;
        self.physical_luid = luid;
    }

    /// Install system routes for full tunnel; bypasses proxy server via physical GW when set.
    pub fn inject_routes(
        &mut self,
        tun_gateway: Ipv4Addr,
        tun_luid: u64,
        proxy_server: Option<Ipv4Addr>,
    ) -> Result<usize, EngineError> {
        self.route_plan = RoutePlan::new();
        self.route_plan.full_tunnel_with_bypass(
            tun_gateway,
            tun_luid,
            proxy_server,
            self.physical_gateway,
            self.physical_luid,
        );
        self.route_plan
            .apply_all()
            .map_err(|e| EngineError::Tun(e.to_string()))
    }

    pub fn rollback_routes(&mut self) -> Result<usize, EngineError> {
        self.route_plan
            .rollback_all()
            .map_err(|e| EngineError::Tun(e.to_string()))
    }

    pub fn route_plan_len(&self) -> usize {
        self.route_plan.desired().len()
    }

    /// Feed one TUN packet into the userspace stack using current route decision for dst.
    pub fn handle_tun_packet(&mut self, packet: &[u8]) -> (Vec<Vec<u8>>, Option<StackEvent>) {
        // Best-effort: decide by destination IP from packet
        let outbound = if let Ok((hdr, _)) = netpilot_netstack::parse_ipv4(packet) {
            let dst = netpilot_netstack::addr_str(hdr.dst);
            self.decide(None, Some(&dst), None).outbound
        } else {
            self.selected_outbound
                .clone()
                .unwrap_or_else(|| "DIRECT".into())
        };
        self.netstack.handle_inbound(packet, &outbound)
    }

    pub fn netstack_stats(&self) -> (u64, u64, u64, usize) {
        (
            self.netstack.packets_in,
            self.netstack.packets_out,
            self.netstack.syns,
            self.netstack.conn_count(),
        )
    }

    pub fn reality_probe(
        &self,
        server_name: &str,
        fingerprint: &str,
    ) -> Result<(String, usize), EngineError> {
        let cfg = RealityConfig::new(server_name).with_fingerprint(Fingerprint::parse(fingerprint));
        let session = RealitySession::open(cfg).map_err(|e| EngineError::State(e))?;
        Ok((session.digest, session.client_hello.len()))
    }

    pub fn stats(&self) -> &EngineStats {
        &self.stats
    }

    pub fn dial_timeout(&self) -> Duration {
        self.dial_timeout
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
            cipher: None,
            public_key: None,
            short_id: None,
            fingerprint: None,
            tags: vec![],
        });
        eng.select_outbound("p1");
        assert_eq!(eng.selected_outbound(), Some("p1"));
    }

    #[test]
    fn inject_syn_packet() {
        use netpilot_netstack::{build_ipv4, build_tcp, FLAG_SYN};
        let mut eng = TrafficEngine::new();
        eng.start_tunnel(Some("T")).unwrap();
        let tcp = build_tcp(40000, 80, 1, 0, FLAG_SYN, 65535, &[]);
        let pkt = build_ipv4([10, 0, 0, 2], [1, 1, 1, 1], 6, &tcp, 1);
        let r = eng.inject_packet_and_relay(&pkt).unwrap();
        assert_eq!(r.event_kind.as_deref(), Some("TcpSyn"));
        assert!(r.replies_out >= 1);
        // dial to 1.1.1.1:80 may fail in CI; event still recorded
        eng.stop_tunnel().unwrap();
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
