# Download rradar.exe from GitHub Releases (Windows x86_64).
param(
    [string]$Tag = "latest",
    [string]$Repo = $(if ($env:RRADAR_REPO) { $env:RRADAR_REPO } else { "taipei49314/receiptradar" }),
    [string]$InstallDir = ""
)

$ErrorActionPreference = "Stop"
$Artifact = "rradar-x86_64-pc-windows-msvc"
if (-not $InstallDir) {
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path $cargoBin) { $InstallDir = $cargoBin }
    else { $InstallDir = Join-Path $env:USERPROFILE "bin" }
}
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

if ($Tag -eq "latest") {
    $base = "https://github.com/$Repo/releases/latest/download"
} else {
    $base = "https://github.com/$Repo/releases/download/$Tag"
}

$tmp = Join-Path $env:TEMP ("rradar-rel-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
try {
    $zip = Join-Path $tmp "$Artifact.zip"
    $sum = Join-Path $tmp "$Artifact.sha256"
    Write-Host "fetch $Artifact.zip ($Tag)"
    Invoke-WebRequest -Uri "$base/$Artifact.zip" -OutFile $zip -UseBasicParsing
    try {
        Invoke-WebRequest -Uri "$base/$Artifact.sha256" -OutFile $sum -UseBasicParsing
        $want = ((Get-Content $sum -Raw) -split '\s+')[0].ToLowerInvariant()
        $got = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($want -ne $got) { throw "checksum mismatch: expected $want got $got" }
        Write-Host "checksum ok"
    } catch {
        Write-Warning "checksum skip/fail: $_"
    }
    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $exe = Get-ChildItem -Path $tmp -Recurse -Filter "rradar.exe" | Select-Object -First 1
    if (-not $exe) { throw "rradar.exe not found in archive" }
    $dest = Join-Path $InstallDir "rradar.exe"
    Copy-Item $exe.FullName -Destination $dest -Force
    Write-Host "installed $dest"
    Write-Host "ensure $InstallDir is on PATH"
    & $dest version --long
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
