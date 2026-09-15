//! Local SOCKS5 inbound: accept → rules decide → outbound dial → bidirectional copy.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use netpilot_outbound::{dial_outbound, DialRequest, OutboundStream};

use crate::TrafficEngine;

#[derive(Debug, Default)]
pub struct InboundStats {
    pub accepted: u64,
    pub active: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub errors: u64,
}

pub struct SocksInbound {
    stop: Arc<AtomicBool>,
    port: u16,
    stats: Arc<Mutex<InboundStats>>,
}

impl SocksInbound {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stats_snapshot(&self) -> InboundStats {
        self.stats
            .lock()
            .map(|g| InboundStats {
                accepted: g.accepted,
                active: g.active,
                bytes_up: g.bytes_up,
                bytes_down: g.bytes_down,
                errors: g.errors,
            })
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Bind 127.0.0.1:`port` (or 0 for ephemeral) and serve SOCKS5 until stop.
pub fn start_socks_inbound(
    engine: Arc<Mutex<TrafficEngine>>,
    port: u16,
) -> Result<SocksInbound, String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let bound = listener.local_addr().map_err(|e| e.to_string())?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(Mutex::new(InboundStats::default()));
    let stop_t = stop.clone();
    let stats_t = stats.clone();

    thread::spawn(move || {
        while !stop_t.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Ok(mut g) = stats_t.lock() {
                        g.accepted = g.accepted.saturating_add(1);
                        g.active = g.active.saturating_add(1);
                    }
                    let eng = engine.clone();
                    let st = stats_t.clone();
                    thread::spawn(move || {
                        let result = handle_socks_client(stream, eng);
                        if let Ok(mut g) = st.lock() {
                            g.active = g.active.saturating_sub(1);
                            match result {
                                Ok((up, down)) => {
                                    g.bytes_up = g.bytes_up.saturating_add(up);
                                    g.bytes_down = g.bytes_down.saturating_add(down);
                                }
                                Err(_) => g.errors = g.errors.saturating_add(1),
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
    });

    Ok(SocksInbound {
        stop,
        port: bound,
        stats,
    })
}

fn handle_socks_client(
    mut client: TcpStream,
    engine: Arc<Mutex<TrafficEngine>>,
) -> Result<(u64, u64), String> {
    let _ = client.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(30)));

    // greeting
    let mut hdr = [0u8; 2];
    client.read_exact(&mut hdr).map_err(|e| e.to_string())?;
    if hdr[0] != 0x05 {
        return Err("not socks5".into());
    }
    let nmethods = hdr[1] as usize;
    let mut methods = vec![0u8; nmethods];
    client.read_exact(&mut methods).map_err(|e| e.to_string())?;
    client.write_all(&[0x05, 0x00]).map_err(|e| e.to_string())?;

    // request
    let mut req = [0u8; 4];
    client.read_exact(&mut req).map_err(|e| e.to_string())?;
    if req[0] != 0x05 {
        return Err("bad socks version".into());
    }
    // CMD: 0x01 CONNECT, 0x03 UDP ASSOCIATE
    match req[1] {
        0x01 => handle_socks_connect(client, engine, req[3]),
        0x03 => handle_socks_udp_associate(client, engine, req[3]),
        _ => {
            let _ = client.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
            Err("unsupported CMD".into())
        }
    }
}

fn handle_socks_connect(
    mut client: TcpStream,
    engine: Arc<Mutex<TrafficEngine>>,
    atyp: u8,
) -> Result<(u64, u64), String> {
    let (host, port) = read_socks_target(&mut client, atyp)?;

    let (outbound_name, profiles) = {
        let g = engine.lock().map_err(|e| e.to_string())?;
        let r = g.decide(Some(&host), None, Some(port));
        (r.outbound, g.profiles().to_vec())
    };

    if outbound_name.eq_ignore_ascii_case("REJECT") {
        let _ = client.write_all(&[0x05, 0x02, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
        return Err("rejected by rules".into());
    }

    let mut dreq = DialRequest::new(&host, port);
    dreq.timeout = Duration::from_secs(10);
    let remote = match dial_outbound(&outbound_name, &profiles, &dreq) {
        Ok((s, _)) => s,
        Err(e) => {
            let _ = client.write_all(&[0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
            return Err(e.to_string());
        }
    };

    client
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .map_err(|e| e.to_string())?;

    pipe_copy(client, remote)
}

fn handle_socks_udp_associate(
    mut client: TcpStream,
    engine: Arc<Mutex<TrafficEngine>>,
    atyp: u8,
) -> Result<(u64, u64), String> {
    // consume client-supplied target (often 0.0.0.0:0)
    let _ = read_socks_target(&mut client, atyp);

    use std::net::UdpSocket;
    let udp = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let local = udp.local_addr().map_err(|e| e.to_string())?;
    let port = local.port();
    // reply BND.ADDR = 127.0.0.1:port
    let mut reply = vec![0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1];
    reply.extend_from_slice(&port.to_be_bytes());
    client.write_all(&reply).map_err(|e| e.to_string())?;

    let eng = engine.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_u = stop.clone();
    let udp_c = udp.try_clone().map_err(|e| e.to_string())?;
    let h = thread::spawn(move || udp_relay_loop(udp_c, eng, stop_u));

    // hold TCP control until client disconnects
    let mut buf = [0u8; 64];
    loop {
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    stop.store(true, Ordering::SeqCst);
    let _ = h.join();
    Ok((0, 0))
}

/// SOCKS5 UDP packet: RSV RSV FRAG ATYP DST.ADDR DST.PORT DATA
fn udp_relay_loop(
    sock: std::net::UdpSocket,
    engine: Arc<Mutex<TrafficEngine>>,
    stop: Arc<AtomicBool>,
) {
    let _ = sock.set_read_timeout(Some(Duration::from_millis(500)));
    let mut buf = [0u8; 65535];
    // client endpoint learned from first packet
    let mut client_addr: Option<std::net::SocketAddr> = None;
    while !stop.load(Ordering::SeqCst) {
        match sock.recv_from(&mut buf) {
            Ok((n, from)) => {
                if n < 4 {
                    continue;
                }
                // outbound from client
                if client_addr.is_none() || client_addr == Some(from) {
                    client_addr = Some(from);
                    if let Some((host, port, hdr_len)) = parse_socks_udp_header(&buf[..n]) {
                        let payload = &buf[hdr_len..n];
                        let (outbound_name, profiles) = {
                            match engine.lock() {
                                Ok(g) => {
                                    let r = g.decide(Some(&host), None, Some(port));
                                    (r.outbound, g.profiles().to_vec())
                                }
                                Err(_) => continue,
                            }
                        };
                        if outbound_name.eq_ignore_ascii_case("REJECT") {
                            continue;
                        }
                        // UDP via direct path: send payload to target and wait reply
                        if let Ok(target) = format!("{host}:{port}").parse::<std::net::SocketAddr>()
                        {
                            if let Ok(relay) = std::net::UdpSocket::bind("0.0.0.0:0") {
                                let _ = relay.set_read_timeout(Some(Duration::from_secs(5)));
                                if relay.send_to(payload, target).is_ok() {
                                    let mut rbuf = [0u8; 65535];
                                    if let Ok((rn, src)) = relay.recv_from(&mut rbuf) {
                                        // encapsulate reply for SOCKS client
                                        let mut out = Vec::with_capacity(10 + rn);
                                        out.extend_from_slice(&[0, 0, 0]); // RSV RSV FRAG
                                        match src {
                                            std::net::SocketAddr::V4(v4) => {
                                                out.push(0x01);
                                                out.extend_from_slice(&v4.ip().octets());
                                                out.extend_from_slice(&v4.port().to_be_bytes());
                                            }
                                            std::net::SocketAddr::V6(v6) => {
                                                out.push(0x04);
                                                out.extend_from_slice(&v6.ip().octets());
                                                out.extend_from_slice(&v6.port().to_be_bytes());
                                            }
                                        }
                                        out.extend_from_slice(&rbuf[..rn]);
                                        let _ = sock.send_to(&out, from);
                                        let _ = profiles; // reserved for SS UDP path
                                        let _ = outbound_name;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
}

fn parse_socks_udp_header(pkt: &[u8]) -> Option<(String, u16, usize)> {
    if pkt.len() < 4 || pkt[2] != 0 {
        return None; // FRAG must be 0
    }
    let atyp = pkt[3];
    match atyp {
        0x01 if pkt.len() >= 10 => {
            let host = format!("{}.{}.{}.{}", pkt[4], pkt[5], pkt[6], pkt[7]);
            let port = u16::from_be_bytes([pkt[8], pkt[9]]);
            Some((host, port, 10))
        }
        0x03 if pkt.len() >= 5 => {
            let len = pkt[4] as usize;
            if pkt.len() < 5 + len + 2 {
                return None;
            }
            let host = String::from_utf8_lossy(&pkt[5..5 + len]).to_string();
            let port = u16::from_be_bytes([pkt[5 + len], pkt[5 + len + 1]]);
            Some((host, port, 5 + len + 2))
        }
        0x04 if pkt.len() >= 22 => {
            let ip = std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&pkt[4..20]).ok()?);
            let port = u16::from_be_bytes([pkt[20], pkt[21]]);
            Some((ip.to_string(), port, 22))
        }
        _ => None,
    }
}

fn read_socks_target(stream: &mut TcpStream, atyp: u8) -> Result<(String, u16), String> {
    match atyp {
        0x01 => {
            let mut b = [0u8; 4 + 2];
            stream.read_exact(&mut b).map_err(|e| e.to_string())?;
            let host = format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
            let port = u16::from_be_bytes([b[4], b[5]]);
            Ok((host, port))
        }
        0x03 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).map_err(|e| e.to_string())?;
            let mut name = vec![0u8; len[0] as usize];
            stream.read_exact(&mut name).map_err(|e| e.to_string())?;
            let mut portb = [0u8; 2];
            stream.read_exact(&mut portb).map_err(|e| e.to_string())?;
            let host = String::from_utf8_lossy(&name).to_string();
            let port = u16::from_be_bytes(portb);
            Ok((host, port))
        }
        0x04 => {
            let mut b = [0u8; 16 + 2];
            stream.read_exact(&mut b).map_err(|e| e.to_string())?;
            // compress to string form
            let host =
                std::net::Ipv6Addr::from(<[u8; 16]>::try_from(&b[..16]).unwrap()).to_string();
            let port = u16::from_be_bytes([b[16], b[17]]);
            Ok((host, port))
        }
        _ => Err(format!("bad atyp {atyp}")),
    }
}

fn pipe_copy(client: TcpStream, remote: OutboundStream) -> Result<(u64, u64), String> {
    let mut c1 = client.try_clone().map_err(|e| e.to_string())?;
    let mut c2 = client;
    match remote {
        OutboundStream::Plain(r) => {
            let mut r1 = r.try_clone().map_err(|e| e.to_string())?;
            let mut r2 = r;
            let h_up = thread::spawn(move || copy_all(&mut c1, &mut r2));
            let down = copy_all(&mut r1, &mut c2);
            let up = h_up.join().unwrap_or(0);
            Ok((up, down))
        }
        OutboundStream::Tls(mut tls) => {
            // Single-threaded alternate copy is complex; use sequential limited for TLS.
            // Prefer: spawn with mutual exclusion — simple blocking half-close style.
            let mut buf = [0u8; 16 * 1024];
            let mut up = 0u64;
            let mut down = 0u64;
            // Set nonblocking-ish timeouts for fairness
            let _ = c2.set_read_timeout(Some(Duration::from_millis(200)));
            let _ = tls.set_read_timeout(Some(Duration::from_millis(200)));
            for _ in 0..100_000 {
                match c2.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tls.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        up += n as u64;
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(_) => break,
                }
                match tls.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if c2.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        down += n as u64;
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(_) => break,
                }
            }
            Ok((up, down))
        }
    }
}

fn copy_all(src: &mut impl Read, dst: &mut impl Write) -> u64 {
    let mut buf = [0u8; 16 * 1024];
    let mut total = 0u64;
    loop {
        match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if dst.write_all(&buf[..n]).is_err() {
                    break;
                }
                total += n as u64;
            }
            Err(_) => break,
        }
    }
    let _ = dst.flush();
    total
}

/// Local HTTP CONNECT inbound for system-proxy / browser traffic.
pub struct HttpInbound {
    stop: Arc<AtomicBool>,
    port: u16,
    stats: Arc<Mutex<InboundStats>>,
}

impl HttpInbound {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stats_snapshot(&self) -> InboundStats {
        self.stats
            .lock()
            .map(|g| InboundStats {
                accepted: g.accepted,
                active: g.active,
                bytes_up: g.bytes_up,
                bytes_down: g.bytes_down,
                errors: g.errors,
            })
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub fn start_http_inbound(
    engine: Arc<Mutex<TrafficEngine>>,
    port: u16,
) -> Result<HttpInbound, String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let bound = listener.local_addr().map_err(|e| e.to_string())?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(Mutex::new(InboundStats::default()));
    let stop_t = stop.clone();
    let stats_t = stats.clone();

    thread::spawn(move || {
        while !stop_t.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if let Ok(mut g) = stats_t.lock() {
                        g.accepted = g.accepted.saturating_add(1);
                        g.active = g.active.saturating_add(1);
                    }
                    let eng = engine.clone();
                    let st = stats_t.clone();
                    thread::spawn(move || {
                        let result = handle_http_client(stream, eng);
                        if let Ok(mut g) = st.lock() {
                            g.active = g.active.saturating_sub(1);
                            match result {
                                Ok((up, down)) => {
                                    g.bytes_up = g.bytes_up.saturating_add(up);
                                    g.bytes_down = g.bytes_down.saturating_add(down);
                                }
                                Err(_) => g.errors = g.errors.saturating_add(1),
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
    });

    Ok(HttpInbound {
        stop,
        port: bound,
        stats,
    })
}

fn handle_http_client(
    mut client: TcpStream,
    engine: Arc<Mutex<TrafficEngine>>,
) -> Result<(u64, u64), String> {
    let _ = client.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(30)));

    // Read request line + headers until blank line
    let mut buf = Vec::with_capacity(1024);
    let mut tmp = [0u8; 1];
    loop {
        client.read_exact(&mut tmp).map_err(|e| e.to_string())?;
        buf.push(tmp[0]);
        if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if buf.len() > 64 * 1024 {
            return Err("http request too large".into());
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let first = text.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("bad request line".into());
    }
    let method = parts[0].to_uppercase();
    let target = parts[1];

    let (host, port) = if method == "CONNECT" {
        parse_host_port(target, 443)?
    } else {
        // Absolute-form URI or Host header — prefer URI
        if let Some(rest) = target.strip_prefix("http://") {
            let hostport = rest.split('/').next().unwrap_or(rest);
            parse_host_port(hostport, 80)?
        } else {
            // Fall back to Host header
            let host_line = text
                .lines()
                .find(|l| l.to_ascii_lowercase().starts_with("host:"))
                .map(|l| l[5..].trim())
                .ok_or_else(|| "no host".to_string())?;
            parse_host_port(host_line, 80)?
        }
    };

    let (outbound_name, profiles, timeout) = {
        let g = engine.lock().map_err(|_| "engine lock".to_string())?;
        let d = g.decide(Some(&host), None, Some(port));
        (d.outbound, g.profiles().to_vec(), g.dial_timeout())
    };

    let mut req = DialRequest::new(&host, port);
    req.timeout = timeout;
    let (mut remote, _) =
        dial_outbound(&outbound_name, &profiles, &req).map_err(|e| e.to_string())?;

    if method == "CONNECT" {
        client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .map_err(|e| e.to_string())?;
    } else {
        // Relay original request
        remote.write_all(&buf).map_err(|e| e.to_string())?;
        let _ = remote.flush();
    }

    relay_bidirectional(client, remote)
}

fn parse_host_port(s: &str, default_port: u16) -> Result<(String, u16), String> {
    let s = s.trim();
    if let Some((h, p)) = s.rsplit_once(':') {
        // Avoid breaking IPv6 without brackets for now
        if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) {
            let port: u16 = p.parse().map_err(|_| "bad port".to_string())?;
            return Ok((h.to_string(), port));
        }
    }
    Ok((s.to_string(), default_port))
}
