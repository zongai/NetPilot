# NetPilot

Windows advanced network traffic-control client.

**Current product version:** `1.0.0-rc.1` (R1 Release Candidate — Final requires human approval, see R1 NP-264).

## Architecture

```
Desktop (WPF / WinUI shell)     ← control plane only
        │  Windows Named Pipe / versioned JSON RPC
        ▼
netpilot-core.exe (Rust)       ← runtime authority
  ├─ subscription & proxy registry
  ├─ rules / routing / dial policy
  ├─ outbound (SOCKS5, HTTP CONNECT, SS, VMess, VLESS, Trojan, REALITY surface)
  ├─ DNS (system / DoH-DoT surfaces / Fake-IP)
  ├─ TUN / Wintun probe + fail-safe
  ├─ system proxy IPC
  ├─ process path matching
  └─ connections / logs / diagnostics
```

Ownership boundaries and non-goals: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
Data plane notes: [`docs/DATAPLANE.md`](docs/DATAPLANE.md).

## Repository layout

| Path | Role |
|------|------|
| `apps/core` | Rust Core binary (`netpilot-core`) |
| `apps/desktop-wpf` | WPF Desktop shell (IPC client; can launch Core) |
| `apps/desktop` | WinUI 3 shell (optional) |
| `crates/*` | Libraries (ipc, config, rules, proxy, dns, tun, subscription, …) |
| `docs/` | Architecture, IPC, task packs (`docs/tasks/NP-*.md`) |
| `scripts/` | CI, smoke, R1 package/install/secret-scan helpers |
| `tests/release/` | R1 acceptance plans and Final gate notes |

## Task packs

| Pack | Range | Status |
|------|-------|--------|
| V4 | NP-001 … NP-120 | Complete (S0–S9) |
| S11 | NP-121 … NP-144 | Subscription add-on |
| V5 | NP-145 … NP-240 | Functional surfaces (F0–F7) |
| R1 | NP-241 … NP-264 | Release engineering → **RC**; Final needs human sign-off |

Agent rules: [`AGENTS.md`](AGENTS.md), [`AGENTS_V5.md`](AGENTS_V5.md), [`AGENTS_R1.md`](AGENTS_R1.md).

## Development

**Pinned toolchain:** Rust **1.98.1** + `rustfmt` + `clippy` (`rust-toolchain.toml`).
Policy: [`docs/TOOLCHAIN.md`](docs/TOOLCHAIN.md). Windows required for full TUN / desktop smoke.

```powershell
rustup show   # expect 1.98.1
./scripts/ci.ps1
./scripts/smoke.ps1
```

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Core IPC

Pipe: `\\.\pipe\netpilot-core`
Protocol: newline-delimited JSON `IpcEnvelope` (versioned).

| Area | Examples |
|------|----------|
| Runtime | `ping`, `health.check`, `runtime.state`, `runtime.shutdown` |
| Subscription | `subscription.list` / `add` / `update` / `remove` |
| Proxy | `proxy.list`, `proxy.upsert`, `proxy.select` |
| Rules | `rules.load`, `rules.decide` |
| Tunnel | `tunnel.status`, `tun.wintun_probe`, `tunnel.start` (admin) |
| System proxy | `system_proxy.query` / `set` / `disable` |
| Observability | `connections.list`, `logs.list`, `netstack.stats` |

- Smoke-only process exit: `NETPILOT_SMOKE_ONLY=1` (**must not** set in production).
- Log level: `NETPILOT_LOG_LEVEL` (`error`|`warn`|`info`|`debug`|`trace`).
- Data root: `NETPILOT_DATA_DIR` or `%LOCALAPPDATA%\NetPilot\data`.

Feature `real-http` (Core default) uses `ureq` for subscription fetch.

## Desktop

1. Prefer `Start-NetPilot.cmd` (starts Core, then WPF).
2. Or start `netpilot-core.exe`, then `NetPilot.Desktop.Wpf.exe`.
3. Offline GUI attempts `TryLaunchCore` next to the Desktop binary.

## Release (R1)

| Workflow | Trigger | Output |
|----------|---------|--------|
| `ci` | push / PR | fmt, check, test, clippy |
| `release` | tag `v*` / main / dispatch | `netpilot-core.exe` |
| `release-full` | tag `v*` / dispatch | **Core + WPF** → `NetPilot-win-x64.zip` |

### Local / lab packaging

```powershell
cargo build -p netpilot-core --release
dotnet publish apps/desktop-wpf/NetPilot.Desktop.Wpf.csproj -c Release -r win-x64 --self-contained true -o dist/desktop
pwsh scripts/package-r1.ps1
pwsh scripts/validate-wintun-r1.ps1
pwsh scripts/first-run-r1.ps1
```

Portable install/uninstall (preserves user data by default):

```powershell
pwsh scripts/install-layout-r1.ps1 -Action Install
pwsh scripts/install-layout-r1.ps1 -Action Uninstall
```

Place trusted `wintun.dll` beside Core for native TUN (admin / human review). See [`docs/TUN.md`](docs/TUN.md), [`docs/UAC_PERMISSIONS_R1.md`](docs/UAC_PERMISSIONS_R1.md).

RC notes: [`RELEASE_NOTES.md`](RELEASE_NOTES.md) · Checklist: [`docs/RELEASE_CHECKLIST_R1.md`](docs/RELEASE_CHECKLIST_R1.md) · Final gate: [`tests/release/final/README.md`](tests/release/final/README.md).

## Security

- Never log passwords, tokens, UUIDs, or sensitive subscription URLs.
- Secret scan helper: `scripts/secret-scan-r1.ps1`
- Review: [`docs/SECURITY_REVIEW_R1.md`](docs/SECURITY_REVIEW_R1.md)

## License

See repository license / workspace package metadata (`MIT OR Apache-2.0`).
