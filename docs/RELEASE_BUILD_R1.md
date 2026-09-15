# R1 Release Build Configuration (NP-241…NP-245)

## Version

- Unified product version: **1.0.0-rc.1** (`VERSION.txt`, workspace package, Desktop WPF)
- Protocol IPC version remains independent (`PROTOCOL_VERSION` in `netpilot-ipc`)

## Cargo release profile

```toml
[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3
strip = "symbols"
debug = false
panic = "abort"
```

## Dev vs release isolation (NP-245)

| Concern | Development | Release |
|---------|-------------|---------|
| HTTP fetch | `real-http` feature (default) | same; mock only when feature off |
| Smoke exit | `NETPILOT_SMOKE_ONLY=1` | **must not** be set |
| Idle deadline | `NETPILOT_IDLE_SECS` (non-Windows) | not used on Windows service loop |
| Log level | `NETPILOT_LOG_LEVEL=debug` | `info` or `warn` |
| Data dir | `NETPILOT_DATA_DIR` override | `%LOCALAPPDATA%\NetPilot` |

Production Core **must not** ship with test credentials, debug IPC endpoints beyond documented ops, or embedded secrets.

## Build commands (Windows CI / human)

```powershell
# Core
cargo build -p netpilot-core --release

# Desktop WPF
dotnet publish apps/desktop-wpf/NetPilot.Desktop.Wpf.csproj -c Release -r win-x64 --self-contained true -o dist/desktop

# Package layout
pwsh scripts/package-r1.ps1
```

Do not record SHA-256 until artifacts exist on disk.
