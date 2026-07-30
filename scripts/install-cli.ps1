# Install rradar CLI to cargo bin (Windows)
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "Building release rradar from source..."
cargo install --path crates/rradar-cli --force --locked
Write-Host "Installed. Ensure %USERPROFILE%\.cargo\bin is on PATH"
& rradar version --long
& rradar doctor
Write-Host "Tip: binary-only install from GitHub Releases → scripts/install-from-release.ps1"
