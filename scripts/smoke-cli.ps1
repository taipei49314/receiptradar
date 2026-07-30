# ReceiptRadar CLI smoke (autonomous loop verification)
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$homeDir = Join-Path $Root "target\smoke-cli"
$env:RRADAR_HOME = $homeDir
$env:RRADAR_DB = Join-Path $homeDir "ledger.db"
$env:RRADAR_FAST_BACKUP = "1"
New-Item -ItemType Directory -Force -Path $homeDir | Out-Null

function Invoke-Rradar([string[]]$a) {
  & cargo run -q -p rradar-cli -- @a
  if ($LASTEXITCODE -ne 0) { throw "rradar failed: $a" }
}

Invoke-Rradar @("init")
Invoke-Rradar @("process", "fixtures/text/familymart_89.txt", "fixtures/text/mrt_taipei.txt", "--confirm", "-q")
Invoke-Rradar @("manual", "--merchant", "測試店", "--amount", "12.5", "--currency", "TWD")
Invoke-Rradar @("list")
Invoke-Rradar @("stats", "--all")
Invoke-Rradar @("export", "json", "-o", (Join-Path $homeDir "export.json"))
Invoke-Rradar @("import", "json", (Join-Path $homeDir "export.json"))
Invoke-Rradar @("doctor")
Write-Host "SMOKE_CLI_OK"
