# NetPilot 1.0.0-rc.1 (R1 Release Candidate notes)

## Highlights

- Functional V5 control plane: Desktop ↔ Named Pipe ↔ Core
- Subscription pipeline (URI / Clash / Sing-box surfaces)
- Proxy registry, rules engine, DNS/Fake-IP stack
- System proxy IPC, process path matching, connection/log surfaces
- TUN/Wintun probe and fail-safe path (driver human-reviewed)

## Package

- Portable: `NetPilot-x64-1.0.0-rc.1.zip` (after `scripts/package-r1.ps1` on Windows with built binaries)
- Setup: optional portable install via `scripts/install-layout-r1.ps1`

## Limitations (RC)

- Not NetPilot 1.0 Final until NP-264 + human approval
- Protocol compatibility subsets for VMess/SSR/REALITY
- SHA-256 sums recorded only after artifacts exist

## Upgrade / data

- Config under `%LOCALAPPDATA%\NetPilot\data` preserved on uninstall of app payload
