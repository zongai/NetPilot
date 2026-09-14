# Data plane (Wintun / outbound / rules→traffic)

## Components

| Crate | Role |
|-------|------|
| `netpilot-os-wintun` | Dynamic `wintun.dll` load + full session FFI (create/open adapter, packet RX/TX) |
| `netpilot-tun` | `WintunSession` lifecycle; native packet IO when DLL present |
| `netpilot-outbound` | Direct / SOCKS5 / HTTP CONNECT / Trojan / VLESS / Shadowsocks dialers |
| `netpilot-engine` | Profiles + rules + tunnel control + `route_and_dial` |

## IPC (Core)

| Operation | Purpose |
|-----------|---------|
| `rules.load` / `rules.decide` | Load Clash-style rules; decide outbound |
| `proxy.upsert` / `proxy.list` / `proxy.select` | Manage nodes used by dial path |
| `traffic.route_dial` | Decide + dial (probe); returns peer/elapsed |
| `tunnel.start` / `tunnel.stop` / `tunnel.status` | Wintun logical/native session |
| `outbound.tcp_probe` | Raw TCP connect probe |
| `tun.wintun_probe` | DLL + export presence |

## Flow

```text
Client request (host, port)
  → TrafficEngine.decide (rules or manual select)
  → dial_outbound (DIRECT | REJECT | profile id)
  → SOCKS5 / HTTP / Trojan+TLS / VLESS+TLS / SS AEAD / Direct TCP
```

Subscription `subscription.update` feeds parsed `ProxyProfile`s into the engine automatically.

## Windows requirements for native TUN

1. Place signed `wintun.dll` next to `netpilot-core.exe`
2. Run Core elevated (adapter create)
3. `tunnel.start` → `native: true` when session opens successfully

Without the DLL, tunnel still enters logical `SessionRunning` for control-plane tests.

## TLS

`netpilot-outbound` defaults to `tls-rustls` for Trojan/VLESS. REALITY full fingerprint is config-complete; production uTLS may be added later.

## Local SOCKS5 inbound

| Operation | Purpose |
|-----------|---------|
| `inbound.socks_start` | `{ "port": 0 }` → listen `127.0.0.1:port` |
| `inbound.socks_stop` | Stop listener |
| `inbound.socks_status` | accepted / active / bytes |

Flow: SOCKS5 client → Core inbound → `rules.decide` → `dial_outbound` → bidirectional relay.

This provides a usable userspace path without a full TUN TCP/IP stack. Combine with system proxy settings or browser SOCKS configuration.
