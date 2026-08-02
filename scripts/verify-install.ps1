# Post-install / release binary verification (local-only).
# Usage (from repo root, after cargo install or install-from-release):
#   powershell -File scripts/verify-install.ps1
#   powershell -File scripts/verify-install.ps1 -Bin path\to\rradar.exe
param(
    [string]$Bin = "",
    [string]$Fixtures = "fixtures"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

if (-not $Bin) {
    $cargo = Join-Path $env:USERPROFILE ".cargo\bin\rradar.exe"
    if (Test-Path $cargo) { $Bin = $cargo }
    elseif (Test-Path "target\release\rradar.exe") { $Bin = (Resolve-Path "target\release\rradar.exe").Path }
    elseif (Get-Command rradar -ErrorAction SilentlyContinue) { $Bin = (Get-Command rradar).Source }
    else { throw "rradar not found — pass -Bin or install first" }
}

if (-not (Test-Path $Fixtures)) {
    Write-Warning "fixtures missing at $Fixtures — release-check will skip process/demo if not found"
}

Write-Host "verify-install | bin=$Bin"
& $Bin version --long
if ($LASTEXITCODE -ne 0) { throw "version failed" }
$vj = & $Bin version --json | ConvertFrom-Json
if ([int]$vj.ledger_schema -lt 4) { throw "ledger_schema $($vj.ledger_schema) < 4 (need soft-delete build)" }
Write-Host "schema v$($vj.ledger_schema) soft_delete=$($vj.soft_delete)"
& $Bin engines
if ($LASTEXITCODE -ne 0) { throw "engines failed" }
& $Bin release-check --fixtures $Fixtures
if ($LASTEXITCODE -ne 0) { throw "release-check failed" }
Write-Host "VERIFY_INSTALL_OK"
