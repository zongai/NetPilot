# NetPilot build artifact layout (NP-154)
# Produces: dist/netpilot-core.exe layout hints for Desktop packaging.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "== cargo build -p netpilot-core --release =="
cargo build -p netpilot-core --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dist = Join-Path $Root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item "target/release/netpilot-core.exe" -Destination $dist -Force
Copy-Item "scripts/Start-NetPilot.cmd" -Destination $dist -ErrorAction SilentlyContinue

Write-Host "Artifacts:"
Get-ChildItem $dist | Format-Table Name, Length
Write-Host "Build layout OK"
