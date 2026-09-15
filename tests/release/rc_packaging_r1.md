# R1 NP-263 — Release Candidate packaging

## Artifacts (when built on Windows)

- `dist/NetPilot-x64-1.0.0-rc.1.zip`
- `VERSION.txt`
- `RELEASE_NOTES.md`
- `SHA256SUMS.txt` — **only after zip exists**

## Commands

```powershell
cargo build -p netpilot-core --release
dotnet publish apps/desktop-wpf/NetPilot.Desktop.Wpf.csproj -c Release -r win-x64 --self-contained true -o dist/desktop
pwsh scripts/package-r1.ps1
pwsh scripts/validate-wintun-r1.ps1
# Get-FileHash dist/NetPilot-x64-1.0.0-rc.1.zip -Algorithm SHA256
```

## Status this environment

Scripts and versioning landed; **binary zip not produced** (no Windows build requested).
