# ReceiptRadar recordable demo (Windows)
# Isolated ledger under target/demo — does not touch your personal ledger.
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$homeDir = Join-Path $Root "target\demo"
New-Item -ItemType Directory -Force -Path $homeDir | Out-Null
$env:RRADAR_HOME = $homeDir
$env:RRADAR_DB = Join-Path $homeDir "ledger.db"
$env:RRADAR_FAST_BACKUP = "1"
$env:RRADAR_FIXTURES = Join-Path $Root "fixtures"

Write-Host "=== ReceiptRadar demo (scripts/demo.ps1) ==="
& cargo run -q -p rradar-cli -- demo --fixtures $env:RRADAR_FIXTURES --db $env:RRADAR_DB
if ($LASTEXITCODE -ne 0) { throw "demo failed" }
Write-Host "DEMO_SCRIPT_OK"
