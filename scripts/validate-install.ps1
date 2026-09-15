# NP-239: Installer / package dependency validation
$ErrorActionPreference = "Stop"
param(
  [string]$PackageDir = "dist"
)

$required = @("netpilot-core.exe")
$missing = @()
foreach ($f in $required) {
  $p = Join-Path $PackageDir $f
  if (-not (Test-Path $p)) { $missing += $f }
}
if ($missing.Count -gt 0) {
  Write-Error "Missing: $($missing -join ', ')"
  exit 1
}
Write-Host "validate-install OK under $PackageDir"
Get-ChildItem $PackageDir | Format-Table Name, Length
