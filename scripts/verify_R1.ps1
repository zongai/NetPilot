$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$tasks = @(Get-ChildItem ./docs/tasks/NP-*.md | Where-Object { $_.BaseName -match '^NP-(24[1-9]|25[0-9]|26[0-4])$' })
if ($tasks.Count -ne 24) { throw "Expected 24 R1 task files, got $($tasks.Count)" }
$m = Get-Content ./R1_MANIFEST.json | ConvertFrom-Json
if (-not $m.verified -or $m.task_count -ne 24) { throw "Manifest failed" }
$required = @(
  "VERSION.txt","RELEASE_NOTES.md","docs/RELEASE_BUILD_R1.md","docs/SECURITY_REVIEW_R1.md",
  "scripts/package-r1.ps1","scripts/secret-scan-r1.ps1","scripts/first-run-r1.ps1",
  "scripts/install-layout-r1.ps1","tests/release/final/README.md"
)
foreach ($f in $required) { if (-not (Test-Path $f)) { throw "Missing $f" } }
$ver = (Get-Content VERSION.txt | Select-Object -First 1).Trim()
if ($ver -ne "1.0.0-rc.1") { throw "VERSION.txt expected 1.0.0-rc.1 got $ver" }
Write-Host "PASS: R1 task pack 24/24 present; version $ver; scripts/docs in place"
Write-Host "NOTE: Binary RC gates still require Windows execution"
