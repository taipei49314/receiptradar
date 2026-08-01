# Recordable terminal demo narrative for GIF / launch video.
# Usage (from repo root, large font, dark theme recommended):
#   powershell -File scripts/record-demo.ps1
#   powershell -File scripts/record-demo.ps1 -PauseSec 1.2
param(
    [double]$PauseSec = 0.8,
    [switch]$SkipDemo
)

$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function Say($msg) {
    Write-Host ""
    Write-Host "══ $msg ══" -ForegroundColor Cyan
    if ($PauseSec -gt 0) { Start-Sleep -Seconds $PauseSec }
}

function Run-Rradar([string[]]$a) {
    & cargo run -q -p rradar-cli -- @a
    if ($LASTEXITCODE -ne 0) { throw "rradar failed: $a" }
}

Say "1/6 Fixture matrix (recordable demo set)"
Run-Rradar @("fixtures", "list", "--fixtures", "fixtures")

Say "2/6 Verify extract totals (mock OCR, offline)"
Run-Rradar @("fixtures", "verify", "--fixtures", "fixtures")

if (-not $SkipDemo) {
    Say "3/6 Full closed-loop demo (isolated ledger)"
    Run-Rradar @("demo", "--fixtures", "fixtures")
} else {
    Write-Host "(skip demo —SkipDemo)"
}

Say "4/6 Engines readiness (ONNX optional)"
Run-Rradar @("engines")

Say "5/6 Year analytics sample (after demo data under demo db)"
# Use default demo path if present
$demoDb = Join-Path $env:APPDATA "receiptradar\demo\ledger.db"
if (Test-Path $demoDb) {
    Run-Rradar @("report", "--year", "2024", "--db", $demoDb)
} else {
    Write-Host "(no demo ledger yet — run without -SkipDemo)"
}

Say "6/6 Install/release gate"
Run-Rradar @("release-check", "--fixtures", "fixtures", "--quiet")

Write-Host ""
Write-Host "RECORD_DEMO_OK — capture this terminal for README GIF" -ForegroundColor Green
Write-Host "Tips: crop to step banners; keep policy line 'No cloud. No account.'"
