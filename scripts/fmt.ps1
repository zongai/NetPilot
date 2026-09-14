# Format the workspace in place (NP-010 developer helper)
$ErrorActionPreference = "Stop"
cargo fmt --all
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "fmt OK"
