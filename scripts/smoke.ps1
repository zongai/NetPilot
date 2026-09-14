# NetPilot lightweight smoke (NP-010)
# Faster than full ci.ps1 — check + test only. See docs/TESTING.md.
$ErrorActionPreference = "Stop"

Write-Host "== toolchain =="
rustc --version

Write-Host "== cargo check --workspace =="
cargo check --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo test --workspace =="
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Smoke OK"
