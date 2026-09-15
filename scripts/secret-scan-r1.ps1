# R1 NP-256 — Sensitive-data leak scan (source tree; no network)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$patterns = @(
  'password\s*=\s*"[^"]{8,}"',
  'api[_-]?key\s*=\s*"[^"]+"',
  'github_pat_[A-Za-z0-9_]+',
  'BEGIN (RSA |OPENSSH )?PRIVATE KEY',
  'AKIA[0-9A-Z]{16}'
)

$exclude = @('target','dist','.git','node_modules')
$hits = @()
Get-ChildItem -Recurse -File | Where-Object {
  $p = $_.FullName
  -not ($exclude | Where-Object { $p -match [regex]::Escape($_) })
} | ForEach-Object {
  $file = $_
  try {
    $text = Get-Content -Raw -ErrorAction SilentlyContinue $file.FullName
    if (-not $text) { return }
    foreach ($pat in $patterns) {
      if ($text -match $pat) {
        # Allow test redaction fixtures
        if ($file.Name -match 'test|redact|fixture|sample') { continue }
        $hits += "$($file.FullName): matched $pat"
      }
    }
  } catch {}
}

if ($hits.Count -gt 0) {
  Write-Host "POTENTIAL SECRETS:"
  $hits | ForEach-Object { Write-Host $_ }
  exit 1
}
Write-Host "secret-scan: no high-confidence secrets in scanned sources"
exit 0
