# NetPilot lightweight smoke (NP-002+)
# Uses pinned toolchain from rust-toolchain.toml. See docs/TOOLCHAIN.md.
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
