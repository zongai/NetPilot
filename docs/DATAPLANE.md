# Data plane

## TUN (Windows native)

1. Load `wintun.dll` beside Core
2. `tunnel.start` → CreateAdapter / StartSession → **LUID**
3. `CreateUnicastIpAddressEntry` for `10.0.0.1/24` on that LUID
4. Optional `routes.inject` full-tunnel + proxy bypass
5. `tunnel.pump` loop: RX → netstack → TX replies → outbound dial/relay

Without DLL, tunnel stays **logical** (inject via `tunnel.inject` hex packets).

## TUN ↔ outbound relay

```text
TUN packet
  → NetStack (SYN → SYN-ACK, track 4-tuple)
  → on SYN: dial_outbound(rules/manual, dst:port) → RelayFlow
  → on DATA: write to OutboundStream
  → poll OutboundStream → inject_tcp_data → TUN send
```

IPC:

| Op | Role |
|----|------|
| `tunnel.start` / `stop` / `status` | Session + LUID + configured_ip |
| `tunnel.pump` | One receive/relay cycle |
| `tunnel.inject` | `{ "hex": "..." }` synthetic packet |
| `routes.inject` / `rollback` | System routes |
| `netstack.stats` | Stack counters |

## Other

- VMess AEAD dialer, REALITY ClientHello fingerprints (see prior notes)
- SOCKS inbound remains alternative userspace path
