# ReceiptRadar — 30s Taiwan daily path (Windows)
# Isolated ledger under target/day — does not touch your personal ledger.
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$homeDir = Join-Path $Root "target\day"
New-Item -ItemType Directory -Force -Path $homeDir | Out-Null
$env:RRADAR_HOME = $homeDir
$env:RRADAR_DB = Join-Path $homeDir "ledger.db"
$env:RRADAR_FIXTURES = Join-Path $Root "fixtures"

Write-Host "=== ReceiptRadar day (scripts/day.ps1) ==="
& cargo run -q -p rradar-cli -- day --fixtures $env:RRADAR_FIXTURES --db $env:RRADAR_DB
if ($LASTEXITCODE -ne 0) { throw "day failed" }
Write-Host "DAY_SCRIPT_OK"
