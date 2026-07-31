# Optional desktop ONNX e2e smoke (not run in default CI — needs weights + ORT).
# Usage (from repo root):
#   powershell -File scripts/smoke-onnx.ps1
$ErrorActionPreference = "Stop"
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$env:RRADAR_MODELS_DIR = Join-Path $Root "models"
$env:ORT_VERSION = if ($env:ORT_VERSION) { $env:ORT_VERSION } else { "1.22.0" }

Write-Host "=== smoke-onnx: fetch models + ORT $env:ORT_VERSION ==="
powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Root "tools\fetch-models.ps1") -FetchOrt

Write-Host "=== smoke-onnx: build --features onnx --release ==="
cargo build -p rradar-cli --features onnx --release
$rr = Join-Path $Root "target\release\rradar.exe"

Write-Host "=== smoke-onnx: models verify ==="
& $rr models verify
if ($LASTEXITCODE -ne 0) { throw "models verify failed" }

Write-Host "=== smoke-onnx: pixel path with sidecar (no ONNX required for this step) ==="
& $rr process (Join-Path $Root "fixtures\images\familymart_photo.png") --explain
if ($LASTEXITCODE -ne 0) { throw "sidecar image process failed" }

$img = Join-Path $Root "fixtures\images\receipt_en_total89.png"
Write-Host "=== smoke-onnx: real ONNX on $img ==="
& $rr process $img --engine onnx --explain
if ($LASTEXITCODE -ne 0) { throw "onnx process failed" }

Write-Host "SMOKE_ONNX_OK"
