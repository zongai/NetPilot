# R1 NP-251 — First-run initialization
$ErrorActionPreference = "Stop"
$dataRoot = Join-Path $env:LOCALAPPDATA "NetPilot\data"
foreach ($d in @("config","logs","cache","run")) {
  New-Item -ItemType Directory -Force -Path (Join-Path $dataRoot $d) | Out-Null
}
$cfg = Join-Path $dataRoot "config\config.json"
if (-not (Test-Path $cfg)) {
  @'
{
  "schema_version": 1,
  "proxies": [],
  "groups": [],
  "general": {
    "system_proxy": false,
    "allow_lan": false
  }
}
'@ | Set-Content $cfg -Encoding UTF8
  Write-Host "Created default config: $cfg"
} else {
  Write-Host "Config exists (not overwritten): $cfg"
}
$envFile = Join-Path $dataRoot "run\env.defaults"
@"
NETPILOT_DATA_DIR=$dataRoot
NETPILOT_LOG_LEVEL=info
"@ | Set-Content $envFile -Encoding UTF8
Write-Host "First-run OK"
