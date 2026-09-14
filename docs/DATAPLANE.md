# Data plane

## Components

| Crate | Role |
|-------|------|
| `netpilot-os-wintun` | Dynamic `wintun.dll` FFI + packet IO |
| `netpilot-os-route` | System route inject/rollback (Windows IP Helper; logical elsewhere) |
| `netpilot-netstack` | Userspace IPv4/TCP/UDP parse, SYN-ACK, connection table |
| `netpilot-outbound` | Direct / SOCKS5 / HTTP / Trojan / VLESS / VMess / Shadowsocks |
| `netpilot-transport-reality` | REALITY config + uTLS-style ClientHello fingerprints |
| `netpilot-engine` | Rules → dial, TUN, routes, netstack, SOCKS inbound |

## IPC

| Operation | Purpose |
|-----------|---------|
| `routes.inject` | Full-tunnel plan + optional proxy /32 bypass |
| `routes.rollback` | Remove applied routes |
| `netstack.stats` | packets_in/out, syns, connections |
| `reality.fingerprint` | Build ClientHello template; return digest |
| `traffic.route_dial` | Decide + dial |
| `tunnel.*` / `inbound.socks_*` / `proxy.*` | As before |

## Route injection

```text
optional: proxy_server/32 → physical_gateway (keep path to node)
default 0.0.0.0/0 → tun_gateway (metric 5)
```

Windows uses `CreateIpForwardEntry2` / `DeleteIpForwardEntry2`. Requires elevation.

## Userspace stack

TUN IPv4 packet → parse → TCP SYN generates SYN-ACK + conn table entry → engine maps flow to outbound name from rules. Full bidirectional L7 relay over TUN still pairs with outbound streams in subsequent work; SOCKS inbound remains the primary userspace path.

## VMess

AEAD request header (auth id + encrypted length/body) via `netpilot-protocol-vmess`, dialed by `outbound::dial_vmess` (optional TLS).

## REALITY fingerprints

Profiles: chrome / firefox / safari / ios / android / edge. Templates set cipher suite + extension **order** for ClientHello. Not a complete X25519 REALITY crypto handshake.
