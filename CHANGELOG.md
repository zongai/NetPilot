# Changelog

All notable changes to NetPilot are documented here.  
Summaries cover the span **between successful CI builds** on `main`.

Format: [Keep a Changelog](https://keepachangelog.com/)-inspired, versions aligned with workspace `0.1.0` until the first tagged release.

---

## [1.0.0-rc.3] — 2026-09-15

### Fixed

- Desktop Named Pipe IPC: serialize requests + match `request_id` (fixes Proxies/Rules/Subscriptions cross-wired payloads)

---

## [1.0.0-rc.2] — 2026-09-15

**Integration build** after RC.1: real UI↔Core paths + connectivity + TUN pump.

### Added

- **NP-INTEGRATION-001**: `rules.decide` canonical payload; UI sends domain/port; InvalidInput taxonomy tests
- **NP-INTEGRATION-002**: `proxy.upsert` schema + UI add node → `proxy.list`
- **NP-INTEGRATION-003**: `subscription.update` optional `body` → decode/parse → `ProxyProfile` → engine → Proxies
- **`proxy.connectivity`**: real dial → TLS → HTTPS GET stages (not IPC ping)
- **TUN pipeline**: `tunnel.start` auto_route/auto_pump; pump registers **Connections** on TcpSyn

### Changed

- Diagnostics/Home show real Core envelopes; Settings labels `ping` as IPC-only
- IPC protocol docs for rules/proxy/subscription/connectivity/tunnel

### Notes

- `Native Wintun: true` requires Windows + `wintun.dll` beside Core (admin)
- Live HTTPS DIRECT path verified in CI-capable network tests

---

## [1.0.0-rc.1] — 2026-09-15

**R1 Release Candidate** (not Final — NP-264 requires human approval).

### Added

- V5 functional surfaces NP-145…240 (IPC handshake/dispatcher, proxy registry/runtime, DNS runtime, connection/log store, Desktop rules/settings wiring)
- R1 release engineering NP-241…264: unified version `1.0.0-rc.1`, release profile, package/install/first-run/secret-scan scripts, security & UAC docs, acceptance plans
- Full package workflow `release-full` (Core + WPF zip)

### Changed

- Workspace product version → `1.0.0-rc.1`
- README rewritten for V5/R1 layout and IPC surface

### Notes

- SHA-256 recorded after artifacts exist on Windows CI
- TUN/Wintun elevated path remains human-reviewed

---

## [0.1.6] — 2026-09-15

**Full package:** Core + WPF GUI (`NetPilot-win-x64.zip`) via `release-full`.

### Added since 0.1.5

- System proxy auto-switch + IPC (`system_proxy.*`); SOCKS start with `system_proxy: true`
- VMess AEAD / SSR stream+auth / REALITY X25519+fingerprint ClientHello (compatibility-oriented)
- SOCKS5 UDP ASSOCIATE relay; Windows live process path/PID resolver
- TUN↔outbound relay, route injection, netstack, full outbound surface
- CI green on fmt/check/test/clippy; release exe pipeline

### Notes

- Protocol wire formats are subsets; REALITY is auth+fingerprint not full XTLS-REALITY product stack
- UDP to remote proxy (SS/VMess UDP) still staged

---

## [0.1.0] — 2026-09-14

**GitHub Release:** https://github.com/zongai/NetPilot/releases/tag/v0.1.0  
**Artifact:** `netpilot-core.exe` (Windows x64)

First formal release tag. End-to-end task track complete (**NP-001 … NP-120**). CI green on Windows and Ubuntu (`fmt` / `check` / `test` / `clippy`).

### Added

#### Platform & process (S0–S1)

- Workspace, toolchain pin **Rust 1.98.1**, CI baseline, agent workflow docs
- `netpilot-core` binary skeleton and `CoreRuntime` state machine / lifecycle
- Named-pipe IPC envelope, router, health/readiness, timeouts, error map

#### Config & proxy (S2)

- Config load / normalize / validate / migrate pipeline
- Proxy profiles, groups, health, lifecycle, endpoint validation, secret redaction
- Atomic `RuntimeConfig` apply with generation + rollback

#### Rules & routing (S3)

- Clash-like rule parser; domain / IP / port / process / network matchers
- Priority-ordered `RuleIndex`, `RoutingEngine`, explanations, conformance fixtures
- `RuleSet` model, loader, subscription lifecycle, cache, integrity fingerprint, atomic updater

#### Desktop shell (S4)

- WinUI 3 shell, navigation, view-models, `IIpcService` loopback, smoke tags
- User-facing error model; UI operation name constants in IPC

#### TUN (S5)

- `TunProvider` / `MockTunProvider`, device lifecycle, IP config, routes
- Packet ingress/egress pipelines, TCP/UDP intercept hooks, bypass policy
- Rollback/cleanup, mock E2E harness, Wintun feasibility + session wiring surface

#### DNS (S6)

- Resolver abstraction, system probe, UDP/TCP/DoT/DoH transport skeletons
- Cache, Fake-IP (`198.18.0.0/16`), routing policy, leak guard, metrics, fixtures

#### Process attribution (S7)

- Process identity, path matching, PID tracker, process-aware rule matching

#### Diagnostics (S8)

- Connection manager / inspector API, probes, reports
- Script sandbox boundary with resource limits

#### Protocols & transports (S9)

- Shared `ProtocolId` / `TransportId`, URI + subscription parsers, adapter manager
- Shadowsocks / SS2022 / SSR / VMess / VLESS (+ REALITY/Vision) / Trojan configs
- TCP, TLS, WebSocket, HTTP/2, gRPC, REALITY transport surfaces
- Protocol × transport compatibility matrix

### S11 Airport Subscription

- `netpilot-subscription`: fetch/cache/rollback, Base64 + URI/Clash/Sing-box parsers
- Normalize to `ProxyProfile`, dedup/filter/rename/groups, userinfo, desktop shell

### Release engineering

- Matrix CI: `windows-latest` + `ubuntu-latest`
- Release workflow: Windows **release** build of `netpilot-core.exe` + artifact upload

### Security notes

- Secrets redacted in proxy/log paths; script sandbox denies network by default
- No production Wintun/TLS FFI linked in CI (mock / config surfaces only)

---

## Unreleased

### S11 Airport Subscription

- crates/subscription (netpilot-subscription) NP-121..NP-144
- Mock HTTP fetcher, Base64 decoder, URI/Clash/Sing-box parsers
- Normalize to ProxyProfile, fingerprint dedup, filters, groups, userinfo
- Desktop SubscriptionPage.xaml + ViewModel shell

- Load `wintun.dll` at runtime on Windows (feature-gated)
- Real rustls / platform TLS sessions behind `TlsTransport`
- Desktop MSBuild packaging and signed installer

## Unreleased

### Added
- System route injection (`netpilot-os-route`) with full-tunnel + proxy bypass plan
- Userspace IPv4/TCP/UDP stack (`netpilot-netstack`) with SYN-ACK and connection table
- VMess AEAD outbound dialer
- REALITY uTLS-style ClientHello fingerprint profiles (chrome/firefox/safari/ios/android/edge)
- IPC: `routes.inject`, `routes.rollback`, `netstack.stats`, `reality.fingerprint`

### Added (gap fill)
- WebSocket upgrade + binary frames for VLESS/VMess over WS
- ShadowsocksR dial subset
- HTTP/2 preface/settings + gRPC config surface
- IPC: connections.list, config.load, dns.resolve, tunnel.pump_loop_start/stop
- Desktop WPF binds proxy.list / subscription.list / connections.list / tunnel.status
