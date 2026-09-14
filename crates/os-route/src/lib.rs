//! System route injection (Windows IP Helper; logical planner on other OS).
//!
//! Applies/removes IPv4 routes associated with a TUN LUID or interface index.
//! Non-Windows builds keep an in-memory plan for tests.

#![cfg_attr(not(windows), allow(dead_code))]
#![cfg_attr(windows, allow(unsafe_code))]

use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    Unsupported,
    Invalid(String),
    Api(String),
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Unsupported"),
            Self::Invalid(m) => write!(f, "Invalid({m})"),
            Self::Api(m) => write!(f, "Api({m})"),
        }
    }
}

impl std::error::Error for RouteError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemRoute {
    /// Destination prefix, e.g. `0.0.0.0` or `1.2.3.4`.
    pub destination: Ipv4Addr,
    pub prefix_len: u8,
    pub next_hop: Ipv4Addr,
    pub metric: u32,
    /// Interface LUID (Windows) when known; 0 = let OS choose.
    pub interface_luid: u64,
}

impl SystemRoute {
    pub fn default_via(gateway: Ipv4Addr, metric: u32, luid: u64) -> Self {
        Self {
            destination: Ipv4Addr::UNSPECIFIED,
            prefix_len: 0,
            next_hop: gateway,
            metric,
            interface_luid: luid,
        }
    }

    pub fn host(dest: Ipv4Addr, next_hop: Ipv4Addr, metric: u32, luid: u64) -> Self {
        Self {
            destination: dest,
            prefix_len: 32,
            next_hop,
            metric,
            interface_luid: luid,
        }
    }

    pub fn parse_cidr(cidr: &str) -> Result<(Ipv4Addr, u8), RouteError> {
        let (ip, plen) = match cidr.split_once('/') {
            Some((a, b)) => (a, b),
            None => (cidr, "32"),
        };
        let addr: Ipv4Addr = ip
            .parse()
            .map_err(|_| RouteError::Invalid(format!("bad ip {ip}")))?;
        let prefix: u8 = plen
            .parse()
            .map_err(|_| RouteError::Invalid(format!("bad prefix {plen}")))?;
        if prefix > 32 {
            return Err(RouteError::Invalid("prefix > 32".into()));
        }
        Ok((addr, prefix))
    }
}

/// Planned routes + apply/rollback bookkeeping.
#[derive(Debug, Default)]
pub struct RoutePlan {
    desired: Vec<SystemRoute>,
    applied: Vec<SystemRoute>,
}

impl RoutePlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn desired(&self) -> &[SystemRoute] {
        &self.desired
    }

    pub fn applied(&self) -> &[SystemRoute] {
        &self.applied
    }

    pub fn push(&mut self, route: SystemRoute) {
        if let Some(i) = self
            .desired
            .iter()
            .position(|r| r.destination == route.destination && r.prefix_len == route.prefix_len)
        {
            self.desired[i] = route;
        } else {
            self.desired.push(route);
        }
    }

    /// Full-tunnel: default route + optional /32 bypass for proxy server.
    pub fn full_tunnel_with_bypass(
        &mut self,
        tun_gateway: Ipv4Addr,
        tun_luid: u64,
        proxy_server: Option<Ipv4Addr>,
        physical_gateway: Option<Ipv4Addr>,
        physical_luid: u64,
    ) {
        if let (Some(server), Some(gw)) = (proxy_server, physical_gateway) {
            self.push(SystemRoute::host(server, gw, 1, physical_luid));
        }
        self.push(SystemRoute::default_via(tun_gateway, 5, tun_luid));
    }

    pub fn apply_all(&mut self) -> Result<usize, RouteError> {
        let mut n = 0;
        for r in self.desired.clone() {
            install_route(&r)?;
            self.applied.push(r);
            n += 1;
        }
        Ok(n)
    }

    pub fn rollback_all(&mut self) -> Result<usize, RouteError> {
        let mut n = 0;
        while let Some(r) = self.applied.pop() {
            let _ = remove_route(&r);
            n += 1;
        }
        Ok(n)
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::mem::{size_of, zeroed};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NetLuid {
        value: u64,
    }

    #[repr(C)]
    struct IpAddressPrefix {
        prefix: SockAddrInet,
        prefix_length: u8,
    }

    #[repr(C)]
    struct SockAddrInet {
        family: u16,
        // IPv4 sockaddr layout simplified
        port: u16,
        addr: u32,
        zero: [u8; 8],
    }

    #[repr(C)]
    struct MibIpForwardRow2 {
        interface_luid: NetLuid,
        interface_index: u32,
        destination_prefix: IpAddressPrefix,
        next_hop: SockAddrInet,
        site_prefix_length: u8,
        valid_lifetime: u32,
        preferred_lifetime: u32,
        metric: u32,
        protocol: u32,
        loopback: u8,
        autoconfigure_address: u8,
        publish: u8,
        immortal: u8,
        age: u32,
        origin: u32,
    }

    const AF_INET: u16 = 2;
    const MIB_IPPROTO_NETMGMT: u32 = 3;

    #[link(name = "iphlpapi")]
    extern "system" {
        fn CreateIpForwardEntry2(row: *const MibIpForwardRow2) -> u32;
        fn DeleteIpForwardEntry2(row: *const MibIpForwardRow2) -> u32;
    }

    fn make_row(route: &SystemRoute) -> MibIpForwardRow2 {
        unsafe {
            let mut row: MibIpForwardRow2 = zeroed();
            row.interface_luid = NetLuid {
                value: route.interface_luid,
            };
            row.destination_prefix.prefix.family = AF_INET;
            row.destination_prefix.prefix.addr = u32::from(route.destination).to_be();
            row.destination_prefix.prefix_length = route.prefix_len;
            row.next_hop.family = AF_INET;
            row.next_hop.addr = u32::from(route.next_hop).to_be();
            row.metric = route.metric;
            row.protocol = MIB_IPPROTO_NETMGMT;
            row.immortal = 1;
            row.valid_lifetime = 0xffff_ffff;
            row.preferred_lifetime = 0xffff_ffff;
            let _ = size_of::<MibIpForwardRow2>();
            row
        }
    }

    pub fn install_route(route: &SystemRoute) -> Result<(), RouteError> {
        if route.prefix_len > 32 {
            return Err(RouteError::Invalid("prefix".into()));
        }
        let row = make_row(route);
        let code = unsafe { CreateIpForwardEntry2(&row) };
        // 0 = success; 5010 = already exists — treat as ok
        if code == 0 || code == 5010 {
            Ok(())
        } else {
            Err(RouteError::Api(format!(
                "CreateIpForwardEntry2={code} dest={}/{}",
                route.destination, route.prefix_len
            )))
        }
    }

    pub fn remove_route(route: &SystemRoute) -> Result<(), RouteError> {
        let row = make_row(route);
        let code = unsafe { DeleteIpForwardEntry2(&row) };
        if code == 0 || code == 1168 {
            Ok(())
        } else {
            Err(RouteError::Api(format!("DeleteIpForwardEntry2={code}")))
        }
    }
}

#[cfg(windows)]
pub use win::{install_route, remove_route};

#[cfg(not(windows))]
pub fn install_route(_route: &SystemRoute) -> Result<(), RouteError> {
    // Logical success for CI/tests.
    Ok(())
}

#[cfg(not(windows))]
pub fn remove_route(_route: &SystemRoute) -> Result<(), RouteError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_full_tunnel() {
        let mut plan = RoutePlan::new();
        plan.full_tunnel_with_bypass(
            "10.0.0.1".parse().unwrap(),
            1,
            Some("1.2.3.4".parse().unwrap()),
            Some("192.168.1.1".parse().unwrap()),
            2,
        );
        assert_eq!(plan.desired().len(), 2);
        let n = plan.apply_all().unwrap();
        assert_eq!(n, 2);
        assert_eq!(plan.rollback_all().unwrap(), 2);
    }

    #[test]
    fn parse_cidr() {
        let (ip, p) = SystemRoute::parse_cidr("10.0.0.0/8").unwrap();
        assert_eq!(ip, Ipv4Addr::new(10, 0, 0, 0));
        assert_eq!(p, 8);
    }
}
