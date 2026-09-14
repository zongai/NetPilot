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

**Target protocols:** HTTP/HTTPS, SOCKS5, Shadowsocks, Shadowsocks 2022, ShadowsocksR, VMess, VLESS, VLESS+TLS, VLESS+REALITY/Vision, Trojan  

**Target transports:** TCP, UDP, TLS, WebSocket, HTTP/2, gRPC

## Repository layout

| Path | Role |
|------|------|
| `apps/core` | Rust Core binary (`netpilot-core`) |
| `apps/desktop` | WinUI 3 shell (UI only; network stays in Core) |
| `crates/*` | Core libraries (ipc, config, rules, routing, proxy, dns, tun, process, diagnostics, …) |
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
See `docs/TASKS.md` and `docs/tasks/NP-###.md`.  
Agents and contributors must:

1. Read README, architecture/IPC docs, and the **target task** first.
2. Modify **only** the task’s `allowed_paths`.
3. Plan before editing; keep IPC/API compatibility; redact secrets.

## Status

Foundation stage (S0) in progress. Core and protocol crates are workspace skeletons; behavior lands in later NP tasks.
