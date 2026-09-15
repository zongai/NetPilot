# NetPilot V4

Windows advanced network traffic-control client.

## Architecture

```
Desktop.exe (WinUI 3)
        │  Windows Named Pipe / versioned RPC
        ▼
Core.exe (Rust)          ← single source of truth
  ├─ routing & rules
  ├─ proxy lifecycle
  ├─ TUN / packet path
  ├─ DNS (system / DoT / DoH / Fake-IP)
  ├─ process attribution
  └─ connection state & diagnostics
```

Protocol and transport are independent layers.  
REALITY is a security/transport capability, not a standalone protocol.

Ownership boundaries, crate matrix, and non-goals: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

**Target protocols:** HTTP/HTTPS, SOCKS5, Shadowsocks, Shadowsocks 2022, ShadowsocksR, VMess, VLESS, VLESS+TLS, VLESS+REALITY/Vision, Trojan  

**Target transports:** TCP, UDP, TLS, WebSocket, HTTP/2, gRPC

## Repository layout

| Path | Role |
|------|------|
| `apps/core` | Rust Core binary (`netpilot-core`) |
| `apps/desktop` | WinUI 3 shell (UI only; network stays in Core) |
| `crates/*` | Core libraries (ipc, config, rules, routing, proxy, dns, tun, process, diagnostics, outbound, engine, …) |
| `crates/protocols/*` | Protocol adapters (shadowsocks, vmess, vless, trojan, …) |
| `crates/transports/*` | Transport adapters (tcp, tls, websocket, http2, grpc, reality) |
| `docs/` | Architecture, IPC, task index, per-task specs (`docs/tasks/NP-*.md`) |
| `scripts/` | Developer / CI PowerShell helpers |

## Development

**Pinned toolchain:** Rust **1.98.1** + `rustfmt` + `clippy` (`rust-toolchain.toml`).  
Policy details: [`docs/TOOLCHAIN.md`](docs/TOOLCHAIN.md). Windows required for full TUN / desktop smoke.

```powershell
rustup show   # expect 1.98.1

# Format, check, test, lint (CI baseline)
./scripts/ci.ps1

# Lightweight smoke
./scripts/smoke.ps1
```

Or manually:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Task-driven development

Work is split into 120 tasks (NP-001 … NP-120) across stages S0–S9.  
See `docs/TASKS.md`, `docs/INDEX.md`, and `docs/tasks/NP-###.md`.

**Agents (Codex and similar):** follow [`AGENTS.md`](AGENTS.md) and [`docs/AGENT_WORKFLOW.md`](docs/AGENT_WORKFLOW.md).

Contributors and agents must:

1. Read README, architecture/IPC docs, toolchain policy, and the **target task** first.
2. Modify **only** the task’s `allowed_paths`.
3. Plan before editing; keep IPC/API compatibility; redact secrets.
4. Run acceptance commands; report honestly if the environment cannot run them.

## Status

**NP-001 … NP-144 complete** (stages S0–S9 + **S11** subscription).

Data plane (post-S11): `netpilot-outbound` (SOCKS5 / HTTP CONNECT / Trojan / VLESS / SS AEAD dialers + rustls), `netpilot-engine` (rules → dial, TUN session), full Wintun FFI in `netpilot-os-wintun`. See [`docs/DATAPLANE.md`](docs/DATAPLANE.md).

| Stage | Scope |
|-------|--------|
| S0–S1 | Toolchain, CI, Core runtime, IPC |
| S2 | Config + proxy model |
| S3 | Rules / routing engine |
| S4 | WinUI shell + IPC service abstraction |
| S5 | TUN abstraction, bypass, mock E2E, Wintun wiring surface |
| S6 | DNS resolver stack + Fake-IP |
| S7 | Process identity + RuleSet lifecycle |
| S8 | Connections, inspector, diagnostics, script boundary |
| S9 | Protocol/transport adapters + compatibility matrix |
| S11 | Airport subscription fetch/parse/normalize pipeline |

Changelog: [`CHANGELOG.md`](CHANGELOG.md). Architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

Native Wintun requires `wintun.dll` beside Core + elevation. CI uses logical TUN when DLL is absent. Windows release builds produce `netpilot-core.exe` (see Release).

## Release

### GitHub Actions

| Workflow | Trigger | Output |
|----------|---------|--------|
| `ci` | push / PR to `main` | fmt, check, test, clippy (Windows + Ubuntu) |
| `release` | tag `v*`, `workflow_dispatch`, or push to `main` | `netpilot-core.exe` artifact (Windows `release` profile) |

Download artifacts from the Actions run, or create a GitHub Release from a version tag (`v0.1.0`).

### Local release binary

```powershell
cargo build -p netpilot-core --release
# target\release\netpilot-core.exe
```

Place a signed `wintun.dll` next to the Core binary when enabling the Windows TUN path (see `docs/TUN.md`).


## Core IPC (v0.1.3+)

`netpilot-core.exe` stays resident and listens on `\\.\pipe\netpilot-core`.

- Protocol: newline-delimited JSON `IpcEnvelope`
- Ops: `health.check`, `health.ready`, `runtime.state`, `runtime.shutdown`, `ping`
- Smoke exit: set `NETPILOT_SMOKE_ONLY=1`
- GUI: `Start-NetPilot.cmd` starts Core then WPF Desktop


## Subscription IPC (v0.1.4+)

Core exposes:

| Op | Payload |
|----|---------|
| `subscription.list` | — |
| `subscription.add` | `{ "id", "name", "url", "enabled?" }` |
| `subscription.update` | `{ "id", "timeout_secs?" }` |
| `subscription.remove` | `{ "id" }` |

Feature `real-http` (default on Core) uses `ureq` for outbound fetches. Library default remains mock for CI.


## Rules / Outbound / Wintun IPC (v0.1.6+)

| Op | Payload |
|----|---------|
| `rules.load` | `{ "text": "DOMAIN-SUFFIX,google.com,PROXY\nMATCH,DIRECT\n" }` |
| `rules.decide` | `{ "domain"?: "...", "ip"?: "...", "port"?: 443 }` |
| `outbound.tcp_probe` | `{ "host", "port", "timeout_ms?" }` |
| `tun.wintun_probe` | — (loads `wintun.dll` if present beside exe) |

Place official `wintun.dll` next to `netpilot-core.exe` for native probe success.
