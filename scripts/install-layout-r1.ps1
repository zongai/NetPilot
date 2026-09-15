# R1 NP-248/249/250 — Installer data directory layout + portable install/uninstall helpers
# Not a signed MSI; portable layout under Program Files or user-local.
param(
  [ValidateSet("Install","Uninstall")]
  [string]$Action = "Install",
  [string]$Prefix = "$env:LOCALAPPDATA\NetPilot"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $Root "dist/NetPilot-x64"

$data = Join-Path $Prefix "data"
$config = Join-Path $data "config"
$logs = Join-Path $data "logs"
$cache = Join-Path $data "cache"
$run = Join-Path $data "run"
$app = Join-Path $Prefix "app"

function Install-Layout {
  New-Item -ItemType Directory -Force -Path $config,$logs,$cache,$run,$app | Out-Null
  if (-not (Test-Path $stage)) {
    throw "Package stage missing: $stage (run package-r1.ps1 after builds)"
  }
  Copy-Item -Path (Join-Path $stage "*") -Destination $app -Recurse -Force
  # Preserve existing config.json if present
  $cfg = Join-Path $config "config.json"
  if (-not (Test-Path $cfg)) {
    '{"schema_version":1,"proxies":[],"groups":[],"general":{}}' | Set-Content $cfg -Encoding UTF8
  }
  Write-Host "Installed app=$app data=$data (config preserved if existed)"
}

function Uninstall-Layout {
  # Preserve user data by default (NP-250)
  if (Test-Path $app) {
    Remove-Item -Recurse -Force $app
    Write-Host "Removed $app"
  }
  Write-Host "User data kept at $data (delete manually to wipe)"
  Write-Host "Optional wipe: Remove-Item -Recurse -Force `"$data`""
}

if ($Action -eq "Install") { Install-Layout } else { Uninstall-Layout }
