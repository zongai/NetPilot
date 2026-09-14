# Changelog

All notable changes to NetPilot are documented here.  
Summaries cover the span **between successful CI builds** on `main`.

Format: [Keep a Changelog](https://keepachangelog.com/)-inspired, versions aligned with workspace `0.1.0` until the first tagged release.

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
