param([switch]$NoPreview)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$binaryDir = Join-Path $root '.qwen-preview/target/debug'
$binary = Join-Path $binaryDir 'handy.exe'
if (-not (Test-Path $binary)) { throw 'Run .\scripts\setup-qwen-preview.ps1 first.' }
Set-Content -LiteralPath (Join-Path $binaryDir 'portable') -Value 'Handy Portable Mode' -Encoding ascii
$env:HANDY_QWEN_LIVE_PREVIEW = if ($NoPreview) { '0' } else { '1' }
$env:HANDY_DISABLE_UPDATER = '1'
$env:VK_LOADER_LAYERS_DISABLE = '~implicit~'
$env:HF_HOME = Join-Path $binaryDir 'Data/huggingface'
$env:TEMP = Join-Path $root '.qwen-preview/temp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null
Push-Location $root
try {
    & $binary --debug
    if ($LASTEXITCODE -ne 0) { throw "Handy exited with code $LASTEXITCODE" }
} finally { Pop-Location }
