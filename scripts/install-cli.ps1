# Install rradar CLI to cargo bin (Windows)
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

Write-Host "Building release rradar..."
cargo install --path crates/rradar-cli --force --locked
Write-Host "Installed. Ensure %USERPROFILE%\.cargo\bin is on PATH"
& rradar version
& rradar doctor
