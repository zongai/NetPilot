# NetPilot CI baseline (NP-001+)
# Run from repo root on a machine with stable Rust + rustfmt + clippy.
$ErrorActionPreference = "Stop"

Write-Host "== cargo fmt --check =="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo check --workspace =="
cargo check --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo test --workspace =="
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== cargo clippy (deny warnings) =="
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "CI baseline OK"
