# Dual supply-chain gate: Python license scan + optional cargo-deny.
# Usage (repo root):
#   powershell -File scripts/check-supply-chain.ps1
#   powershell -File scripts/check-supply-chain.ps1 -SkipDeny
#   powershell -File scripts/check-supply-chain.ps1 -InstallDeny
param(
    [switch]$SkipDeny,
    [switch]$InstallDeny,
    [switch]$WriteInventory,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

function Info($m) { if (-not $Quiet) { Write-Host $m } }

Info "=== gate 1/2: tools/supply-chain/check_deps.py ==="
$pyArgs = @("tools/supply-chain/check_deps.py")
if ($Quiet) { $pyArgs += "--quiet" }
if ($WriteInventory) { $pyArgs += "--write-inventory" }
& python @pyArgs
if ($LASTEXITCODE -ne 0) { throw "python supply-chain gate failed ($LASTEXITCODE)" }

if ($SkipDeny) {
    Info "=== gate 2/2: cargo-deny SKIPPED (-SkipDeny) ==="
    Write-Host "SUPPLY_CHAIN_DUAL_OK python-only"
    exit 0
}

$deny = Get-Command cargo-deny -ErrorAction SilentlyContinue
if (-not $deny -and $InstallDeny) {
    Info "=== installing cargo-deny 0.20.x ==="
    cargo install cargo-deny --locked --version 0.20.2
    $deny = Get-Command cargo-deny -ErrorAction SilentlyContinue
}

if (-not $deny) {
    Info "=== gate 2/2: cargo-deny not installed (optional) ==="
    Info "    install: cargo install cargo-deny --locked --version 0.20.2"
    Info "    or re-run with -InstallDeny"
    Write-Host "SUPPLY_CHAIN_DUAL_OK python-only (cargo-deny missing)"
    exit 0
}

Info "=== gate 2/2: cargo deny check (deny.toml) ==="
& cargo deny check
if ($LASTEXITCODE -ne 0) { throw "cargo-deny failed ($LASTEXITCODE)" }

Write-Host "SUPPLY_CHAIN_DUAL_OK python+deny"
