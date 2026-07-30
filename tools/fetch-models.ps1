# Fetch hash-pinned OCR models (PR-A05) from HuggingFace SWHL/RapidOCR.
# Windows-native companion to tools/fetch-models.sh.
# Usage:
#   powershell -File tools/fetch-models.ps1
#   powershell -File tools/fetch-models.ps1 -FetchOrt
# Env:
#   RRADAR_MODELS_DIR, RRADAR_MODEL_BASE_URL, ORT_VERSION

param(
    [switch]$FetchOrt,
    [string]$ModelsDir = $(if ($env:RRADAR_MODELS_DIR) { $env:RRADAR_MODELS_DIR } else { Join-Path $PSScriptRoot "..\models" })
)

$ErrorActionPreference = "Stop"
$ModelsDir = [System.IO.Path]::GetFullPath($ModelsDir)
$OrtDir = Join-Path $ModelsDir "ort"
New-Item -ItemType Directory -Force -Path $ModelsDir, $OrtDir | Out-Null

$HfBase = if ($env:RRADAR_MODEL_BASE_URL) { $env:RRADAR_MODEL_BASE_URL.TrimEnd('/') } else { "https://huggingface.co/SWHL/RapidOCR/resolve/main" }

$Files = @(
    @{ Name = "ch_PP-OCRv4_det_infer.onnx"; Rel = "PP-OCRv4/ch_PP-OCRv4_det_infer.onnx" },
    @{ Name = "ch_PP-OCRv4_rec_infer.onnx"; Rel = "PP-OCRv4/ch_PP-OCRv4_rec_infer.onnx" },
    @{ Name = "ch_ppocr_mobile_v2.0_cls_infer.onnx"; Rel = "PP-OCRv1/ch_ppocr_mobile_v2.0_cls_infer.onnx" }
)

$Manifest = Join-Path $ModelsDir "manifest.sha256"
if (-not (Test-Path $Manifest)) {
    @"
# sha256  filename  (fill after first trusted download; fetch verifies when non-comment)
# Generate: Get-FileHash models\*.onnx -Algorithm SHA256
"@ | Set-Content -Path $Manifest -Encoding utf8
    Write-Host "created $Manifest (hashes optional until you pin them)"
}

function Get-Sha256([string]$Path) {
    (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Test-ManifestPin([string]$Name, [string]$Path) {
    if (-not (Test-Path $Manifest)) { return }
    $lines = Get-Content $Manifest | Where-Object { $_ -match '^[0-9a-fA-F]{64}\s+' }
    foreach ($line in $lines) {
        $parts = $line -split '\s+', 2
        if ($parts.Count -lt 2) { continue }
        $want = $parts[0].ToLowerInvariant()
        $file = $parts[1].Trim()
        if ($file -eq $Name) {
            $got = Get-Sha256 $Path
            if ($got -ne $want) {
                throw "hash mismatch for $Name : expected $want got $got"
            }
            Write-Host "verified $Name"
            return
        }
    }
}

foreach ($f in $Files) {
    $dest = Join-Path $ModelsDir $f.Name
    if (Test-Path $dest) {
        Write-Host "exists $($f.Name)"
    } else {
        if ($HfBase -like "*huggingface.co*") {
            $url = "$HfBase/$($f.Rel)"
        } else {
            $url = "$HfBase/$($f.Name)"
        }
        Write-Host "fetch $($f.Name)"
        Write-Host "  from $url"
        Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
    }
    Test-ManifestPin -Name $f.Name -Path $dest
}

if ($FetchOrt -or $env:RRADAR_FETCH_ORT -eq "1") {
    $OrtVer = if ($env:ORT_VERSION) { $env:ORT_VERSION } else { "1.20.1" }
    $zipName = "onnxruntime-win-x64-$OrtVer.zip"
    $url = "https://github.com/microsoft/onnxruntime/releases/download/v$OrtVer/$zipName"
    $tmp = Join-Path $env:TEMP "rradar-ort-$OrtVer"
    $zip = Join-Path $tmp $zipName
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    if (-not (Test-Path (Join-Path $OrtDir "onnxruntime.dll"))) {
        Write-Host "fetch ORT $OrtVer"
        Write-Host "  from $url"
        Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
        Expand-Archive -Path $zip -DestinationPath $tmp -Force
        $dll = Get-ChildItem -Path $tmp -Recurse -Filter "onnxruntime.dll" | Select-Object -First 1
        if (-not $dll) { throw "onnxruntime.dll not found in archive" }
        Copy-Item $dll.FullName -Destination $OrtDir -Force
        Get-ChildItem -Path $dll.DirectoryName -Filter "onnxruntime*.dll" | ForEach-Object {
            Copy-Item $_.FullName -Destination $OrtDir -Force
        }
    } else {
        Write-Host "exists models/ort/onnxruntime.dll"
    }
    $dllPath = Join-Path $OrtDir "onnxruntime.dll"
    Write-Host "ORT ready: $dllPath"
    Write-Host "  (rradar auto-detects models/ort; or set ORT_DYLIB_PATH=$dllPath)"
}

Write-Host "ok — models in $ModelsDir"
Write-Host "next: cargo run -p rradar-cli --features onnx -- process photo.jpg --engine onnx"
