# NetPilot CI baseline (NP-009/NP-010)
# Parity with .github/workflows/ci.yml — run on Windows with rust-toolchain.toml.
# See docs/CI.md and docs/TOOLCHAIN.md.
$ErrorActionPreference = "Stop"

Write-Host "== toolchain =="
rustc --version
cargo --version

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
