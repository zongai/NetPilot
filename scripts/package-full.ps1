# Build Core + Desktop into dist/NetPilot-win-x64 (Windows host required for GUI).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "== cargo build -p netpilot-core --release =="
cargo build -p netpilot-core --release

$Out = Join-Path $Root "dist/NetPilot-win-x64"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
Copy-Item "target/release/netpilot-core.exe" -Destination $Out -Force

Write-Host "== dotnet publish Desktop =="
Push-Location (Join-Path $Root "apps/desktop")
dotnet publish NetPilot.Desktop.csproj -c Release -r win-x64 --self-contained true -o (Join-Path $Out "desktop-publish")
Pop-Location
Copy-Item -Path (Join-Path $Out "desktop-publish/*") -Destination $Out -Recurse -Force
Remove-Item -Recurse -Force (Join-Path $Out "desktop-publish") -ErrorAction SilentlyContinue

@"
NetPilot Windows x64
- netpilot-core.exe
- NetPilot.Desktop.exe (+ WinAppSDK deps)
"@ | Set-Content (Join-Path $Out "README.txt")

Write-Host "Package staged at $Out"
Get-ChildItem $Out | Format-Table Name, Length
