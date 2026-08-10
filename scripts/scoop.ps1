# ReceiptRadar — scoop inbox into today (Windows)
# Copies curated fixtures into an isolated inbox, then runs `rradar scoop`.
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$homeDir = Join-Path $Root "target\scoop"
$inbox = Join-Path $homeDir "inbox"
New-Item -ItemType Directory -Force -Path $inbox | Out-Null
$env:RRADAR_HOME = $homeDir
$env:RRADAR_DB = Join-Path $homeDir "ledger.db"
$env:RRADAR_INBOX = $inbox

$fixtures = Join-Path $Root "fixtures\text"
Copy-Item (Join-Path $fixtures "familymart_89.txt") $inbox -Force
Copy-Item (Join-Path $fixtures "bubbletea_50lan_tw.txt") $inbox -Force

Write-Host "=== ReceiptRadar scoop (scripts/scoop.ps1) ==="
& cargo run -q -p rradar-cli -- scoop --quiet
if ($LASTEXITCODE -ne 0) { throw "scoop failed" }
Write-Host "SCOOP_SCRIPT_OK"
