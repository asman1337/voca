# VOCA — Whisper model downloader (PowerShell)
# Usage: .\scripts\download_model.ps1 [-Tier tiny|base|small|medium]
# Default tier: base

param(
    [ValidateSet("tiny","base","small","medium")]
    [string]$Tier = "base"
)

$ErrorActionPreference = "Stop"

$RepoRoot  = Split-Path $PSScriptRoot -Parent
$ModelsDir = Join-Path $RepoRoot "models"
$BaseUrl   = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main"

$ModelFiles = @{
    tiny   = "ggml-tiny.bin"
    base   = "ggml-base.bin"
    small  = "ggml-small.bin"
    medium = "ggml-medium.bin"
}

$Sizes = @{
    tiny   = "~75 MB"
    base   = "~142 MB"
    small  = "~466 MB"
    medium = "~1.5 GB"
}

$Filename = $ModelFiles[$Tier]
$Dest     = Join-Path $ModelsDir $Filename
$Url      = "$BaseUrl/$Filename"

if (Test-Path $Dest) {
    Write-Host "✓ $Filename already exists at $Dest" -ForegroundColor Green
    exit 0
}

if (-not (Test-Path $ModelsDir)) {
    New-Item -ItemType Directory -Path $ModelsDir | Out-Null
}

Write-Host "Downloading $Filename ($($Sizes[$Tier]))..." -ForegroundColor Cyan
Write-Host "URL: $Url"

try {
    $ProgressPreference = 'Continue'
    Invoke-WebRequest -Uri $Url -OutFile $Dest -UseBasicParsing
} catch {
    Write-Host "Download failed: $_" -ForegroundColor Red
    if (Test-Path $Dest) { Remove-Item $Dest }
    exit 1
}

Write-Host ""
Write-Host "✓ Downloaded to: $Dest" -ForegroundColor Green
Write-Host "Set model_path = `"$Dest`" in %APPDATA%\voca\config.toml"
