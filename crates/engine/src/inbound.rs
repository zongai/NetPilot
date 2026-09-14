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
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
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
    if req[0] != 0x05 || req[1] != 0x01 {
        let _ = client.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
        return Err("only CONNECT supported".into());
    }
    let (host, port) = read_socks_target(&mut client, req[3])?;

    // decide + dial
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

    // success reply
    client
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .map_err(|e| e.to_string())?;

    pipe_copy(client, remote)
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
