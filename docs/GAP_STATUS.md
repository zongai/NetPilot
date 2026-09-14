# Implementation gap status (post gap-fill)

## Implemented end-to-end (control + dial path)
- Core resident + Named Pipe IPC
- Subscription fetch (real-http) → ProxyProfile
- Rules load/decide
- Outbound: Direct, SOCKS5, HTTP CONNECT, Trojan, VLESS (±WS), VMess (±WS), SS AEAD, SSR subset
- TUN: Wintun FFI, IP config, route plan, netstack SYN/relay pump
- REALITY ClientHello fingerprint templates
- SOCKS inbound relay
- Config load JSON/YAML into profiles
- DNS resolve via OS getaddrinfo
- Connections list IPC (manager)
- WPF live lists for proxies/subscriptions/connections

## Known remaining limits (not full production parity)
- VMess/SSR/REALITY crypto not bit-identical to every upstream client
- TUN L7 relay depends on continuous `tunnel.pump` / pump_loop
- Windows IP Helper structs simplified vs windows-sys
- WinUI pages still sample-oriented; WPF is primary desktop shell
- No system proxy toggle automation
- UDP associate / full UDP relay incomplete
- Process-based routing needs live process resolver on Windows PID path
