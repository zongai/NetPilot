# R1 NP-247 — Wintun/driver dependency validation (fail-safe; human review for drivers)
$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $Root "dist/NetPilot-x64"

Write-Host "== Wintun dependency check =="
$found = $false
foreach ($p in @(
  (Join-Path $stage "wintun.dll"),
  (Join-Path $Root "third_party/wintun/wintun.dll"),
  (Join-Path $Root "wintun.dll")
)) {
  if (Test-Path $p) {
    Write-Host "FOUND: $p"
    $found = $true
  }
}
if (-not $found) {
  Write-Warning "wintun.dll not found — TUN native mode will probe and degrade safely"
  Write-Host "Human review required before shipping TUN as default-on"
  exit 0
}
Write-Host "Wintun present (still requires admin/UAC on first adapter create)"
exit 0
