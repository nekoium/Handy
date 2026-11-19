param([switch]$PrepareOnly)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$work = Join-Path $root '.qwen-preview'
$backend = Join-Path $work 'upstream'
$revision = 'adfc50210a3f026a655a3e19937d90ef2bf0728c'

function Invoke-Checked {
    $program = $args[0]
    [string[]]$programArgs = $args | Select-Object -Skip 1
    & $program @programArgs
    if ($LASTEXITCODE -ne 0) { throw "$program failed with exit code $LASTEXITCODE" }
}

New-Item -ItemType Directory -Force $work | Out-Null
if (-not (Test-Path (Join-Path $backend '.git'))) {
    if (Test-Path $backend) { throw "Refusing to overwrite $backend; move that directory before retrying." }
    Invoke-Checked git init $backend
    Invoke-Checked git -C $backend remote add origin 'https://github.com/handy-computer/transcribe.cpp.git'
    Invoke-Checked git -C $backend fetch --depth 1 origin $revision
    Invoke-Checked git -C $backend checkout --detach FETCH_HEAD
}
$current = & git -C $backend rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $current -ne $revision) { throw "Backend must be at pinned revision $revision; refusing to modify another checkout." }
$dirty = & git -C $backend status --porcelain --untracked-files=all
if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Backend must be pristine pinned upstream; refusing a modified checkout.' }
Write-Host "Unchanged pinned upstream backend ready: $current"
if ($PrepareOnly) { return }

if ($env:OS -ne 'Windows_NT') { throw 'This launcher supports Windows x64. See QWEN-LIVE-PREVIEW.md for platform limits.' }
foreach ($tool in @('git', 'cmake', 'cargo', 'bun')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing $tool. See BUILD.md and QWEN-LIVE-PREVIEW.md." }
}
if (-not $env:VULKAN_SDK) { throw 'Set VULKAN_SDK to a Vulkan SDK installation before building.' }
$env:TEMP = Join-Path $work 'temp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force $env:TEMP | Out-Null
if (-not $env:CARGO_HOME) { $env:CARGO_HOME = Join-Path $work 'cargo-home' }
if (-not $env:BUN_INSTALL_CACHE_DIR) { $env:BUN_INSTALL_CACHE_DIR = Join-Path $work 'bun-cache' }
$env:CARGO_TARGET_DIR = Join-Path $work 'target'
$env:TRANSCRIBE_DIR = Join-Path $work 'native'
$nativeBuild = Join-Path $work 'native-build'
Invoke-Checked cmake -S $backend -B $nativeBuild -G 'Visual Studio 17 2022' -A x64 -DTRANSCRIBE_VULKAN=ON -DTRANSCRIBE_BUILD_TESTS=OFF -DTRANSCRIBE_BUILD_SHARED=ON -DTRANSCRIBE_INSTALL=ON -DGGML_NATIVE=OFF
Invoke-Checked cmake --build $nativeBuild --config Release --target transcribe-cli --parallel 8
Invoke-Checked cmake --install $nativeBuild --config Release --prefix $env:TRANSCRIBE_DIR

$vad = Join-Path $root 'src-tauri/resources/models/silero-vad-v6.2-F32.gguf'
if (-not (Test-Path $vad)) {
    New-Item -ItemType Directory -Force (Split-Path -Parent $vad) | Out-Null
    Invoke-WebRequest -Uri 'https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/main/silero-vad-v6.2-F32.gguf' -OutFile "$vad.download"
    Move-Item -LiteralPath "$vad.download" -Destination $vad
}
Push-Location $root
try {
    Invoke-Checked bun install --frozen-lockfile
    Invoke-Checked bun run build
    $env:TAURI_CONFIG = '{"productName":"Handy Qwen Preview","identifier":"local.handy.qwen-preview","build":{"devUrl":null}}'
    Invoke-Checked cargo build --locked --manifest-path src-tauri/Cargo.toml --bin handy
} finally { Pop-Location }
$binaryDir = Join-Path $env:CARGO_TARGET_DIR 'debug'
Set-Content -LiteralPath (Join-Path $binaryDir 'portable') -Value 'Handy Portable Mode' -Encoding ascii
Write-Host 'Build ready. Run .\scripts\run-qwen-preview.ps1, then select a Qwen model and the Live overlay.'
