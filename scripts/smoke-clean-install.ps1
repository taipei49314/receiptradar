# ReceiptRadar clean-install smoke (Windows)
# Installs CLI into an isolated cargo home and runs release-check without relying on the source tree PATH.
# Usage (from repo root):
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/smoke-clean-install.ps1
param(
  [string]$Fixtures = "fixtures"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Test-Path -LiteralPath $Fixtures)) {
  throw "fixtures not found: $Fixtures"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$work = Join-Path $env:TEMP "rradar-clean-install-$stamp"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$cargoHome = Join-Path $work "cargo-home"
$installRoot = Join-Path $work "install"
New-Item -ItemType Directory -Force -Path $cargoHome, $installRoot | Out-Null

$env:CARGO_HOME = $cargoHome
Write-Host "clean-install workdir=$work"

cargo install --path (Join-Path $Root "crates\rradar-cli") --locked --root $installRoot --force
$bin = Join-Path $installRoot "bin\rradar.exe"
if (-not (Test-Path -LiteralPath $bin)) {
  throw "installed binary missing: $bin"
}

& $bin version --long
& $bin version --json
& $bin engines
& $bin release-check --fixtures (Join-Path $Root $Fixtures) --quiet
& $bin doctor

Write-Host "SMOKE_CLEAN_INSTALL_OK bin=$bin"
Write-Host "workdir=$work"
