# R1 NP-246/248/263 — Runtime dependency collection + package layout + RC packaging
# Does NOT build; stages existing artifacts into dist/NetPilot-x64.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$ver = (Get-Content VERSION.txt -ErrorAction SilentlyContinue | Select-Object -First 1).Trim()
if (-not $ver) { $ver = "1.0.0-rc.1" }

$stage = Join-Path $Root "dist/NetPilot-x64"
New-Item -ItemType Directory -Force -Path $stage | Out-Null

$core = Join-Path $Root "target/release/netpilot-core.exe"
$desk = Join-Path $Root "dist/desktop"
$copied = @()

if (Test-Path $core) {
  Copy-Item $core -Destination $stage -Force
  $copied += "netpilot-core.exe"
} else {
  Write-Warning "Missing $core — run cargo build -p netpilot-core --release first"
}

if (Test-Path $desk) {
  Copy-Item -Path (Join-Path $desk "*") -Destination $stage -Recurse -Force
  $copied += "desktop/*"
} else {
  Write-Warning "Missing $desk — run dotnet publish for WPF first"
}

# Optional Wintun DLL if present next to scripts or third_party
$wintunCandidates = @(
  (Join-Path $Root "third_party/wintun/wintun.dll"),
  (Join-Path $Root "wintun.dll")
)
foreach ($w in $wintunCandidates) {
  if (Test-Path $w) {
    Copy-Item $w -Destination $stage -Force
    $copied += "wintun.dll"
    break
  }
}

Copy-Item (Join-Path $Root "scripts/Start-NetPilot.cmd") -Destination $stage -ErrorAction SilentlyContinue
Copy-Item (Join-Path $Root "VERSION.txt") -Destination $stage -ErrorAction SilentlyContinue

@"
NetPilot $ver (R1 package layout)
Files staged: $($copied -join ', ')
Data directory (runtime): %LOCALAPPDATA%\NetPilot
Start: Start-NetPilot.cmd or NetPilot.Desktop.Wpf.exe (launches Core when offline)
"@ | Set-Content -Path (Join-Path $stage "README.txt") -Encoding UTF8

Write-Host "Staged: $stage"
Get-ChildItem $stage | Format-Table Name, Length

$zip = Join-Path $Root "dist/NetPilot-x64-$ver.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
if ((Get-ChildItem $stage -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0) {
  Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip
  Write-Host "Zip: $zip"
} else {
  Write-Warning "Stage empty — zip not created"
}
