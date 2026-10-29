param(
    [Parameter(Position = 0)] [string] $Recipe = 'check',
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $RecipeArgs
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
$logs = Join-Path $root 'target/logs'
New-Item -ItemType Directory -Force -Path $logs | Out-Null
if ($Recipe -notmatch '^[a-z][a-z0-9-]*$') { throw 'Invalid recipe name' }
$log = Join-Path $logs "$Recipe.log"
$exitFile = Join-Path $logs "$Recipe.exit"
'RUNNING' | Set-Content -LiteralPath $exitFile
$code = 0
try {
    & {
        $ErrorActionPreference = 'Continue'
        $vclib = "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\lib\x64"
        if (Test-Path $vclib) {
            $winlib = Join-Path $root "target\winsdk-libs"
            $env:LIB = "$winlib;$vclib;$env:LIB"
            $env:PATH = "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\bin\Hostx64\x64;$env:PATH"
        }
        foreach ($ortLib in @((Join-Path $root 'target/whisper-venv/Lib/site-packages/onnxruntime/capi'), (Join-Path $root '.venv/Lib/site-packages/onnxruntime/capi'))) {
            if (Test-Path $ortLib) {
                $env:PATH = "$ortLib;$env:PATH"
                if (!$env:ORT_DYLIB_PATH -and (Test-Path (Join-Path $ortLib 'onnxruntime.dll'))) {
                    $env:ORT_DYLIB_PATH = Join-Path $ortLib 'onnxruntime.dll'
                }
            }
        }
        if (Test-Path "C:\msys64\ucrt64\bin\gcc.exe") {
            $env:CC = "C:\msys64\ucrt64\bin\gcc.exe"
            $env:AR = "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\14.29.30133\bin\Hostx64\x64\lib.exe"
        }

        if ($Recipe -in @('bootstrap','fmt','format','lint','test','build','lock','run','mock','openapi','eval','bench')) { Get-Command cargo -ErrorAction Stop | Out-Null }
        if ($Recipe -like 'train-*') { Get-Command uv -ErrorAction Stop | Out-Null }
        switch ($Recipe) {
            'audio-worker-test' { & python -m unittest discover -s scripts/tests -p 'test_audio_worker.py'; break }
            'models-admit-known' { & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/admit-transferred-models.ps1; break }
            'inspect-whisper' { $env:PYTHONPATH = (Join-Path $root 'target/model-verify-deps-v2'); & python scripts/inspect-whisper-graph.py; break }
            'whisper-deps-install' {
                $env:UV_PYTHON_INSTALL_DIR = Join-Path $root 'target/tools/python'
                $env:UV_PYTHON_BIN_DIR = Join-Path $root 'target/tools/bin'
                if (!(Test-Path (Join-Path $root 'target/whisper-venv/Scripts/python.exe'))) {
                    & uv venv --python 3.14 target/whisper-venv
                    if ($LASTEXITCODE -ne 0) { throw 'Whisper Python environment creation failed' }
                }
                & uv pip install --python (Join-Path $root 'target/whisper-venv/Scripts/python.exe') --only-binary ':all:' -r scripts/whisper-requirements.txt
                break
            }
            'whisper-smoke' { & (Join-Path $root 'target/whisper-venv/Scripts/python.exe') scripts/whisper-runtime-smoke.py; break }
            'python-version' { & python -c 'import sys; print(sys.version)'; break }
            'multimodal-smoke' { & cargo test -p hre-api --test multimodal_real_api --test media -- --ignored --test-threads=1; break }
            'whisper-api-smoke' { & cargo test -p hre-api --test multimodal_real_api test_real_api_whisper_audio_speech_silence_and_invalid -- --ignored --test-threads=1; break }
            'api-test' { & cargo test -p hre-api --tests -- --test-threads=1; break }
            'api-lint' { & cargo clippy -p hre-api --all-targets -- -D warnings; break }
            'integration-smoke' { & cargo test -p hre-api --test model_runtime -- --ignored --test-threads=1; break }
            'bootstrap' { & cargo install just --version 1.40.0 --locked --root target/tools; break }
            'fmt' { & cargo fmt --all -- --check; break }
            'format' { & cargo fmt --all; break }
            'lint' { & cargo clippy --workspace --all-targets -- -D warnings; break }
            'test' { & cargo test --workspace; break }
            'build' { & cargo build --workspace; break }
            'lock' { & cargo generate-lockfile; break }
            'check' {
                foreach ($step in @('fmt', 'lint', 'test')) {
                    & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/just.ps1 $step
                    if ($LASTEXITCODE -ne 0) { throw "$step failed; see target/logs/$step.log" }
                }
                break
            }
            'run' { & cargo run -p hre-api --bin hre-server -- @RecipeArgs; break }
            'mock' { & cargo run -p hre-api --bin hre-server -- --mock @RecipeArgs; break }
            'openapi' { & cargo run -p hre-api --bin hre-openapi -- docs/openapi.json; break }
            'seed' { & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/demo-seed.ps1 @RecipeArgs; break }
            'demo-seed' { & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/demo-seed.ps1 @RecipeArgs; break }
            'eval' { & cargo run -p hre-eval -- @RecipeArgs; break }
            'bench' { & cargo bench --workspace; break }
            'models-verify' { & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/models-verify.ps1; break }
            'models-verify-install' { & python -m pip install --only-binary=:all: --target target/model-verify-deps-v2 onnx==1.20.1; break }
            'models-verify-test' { $env:PYTHONPATH = (Join-Path $root 'target/model-verify-deps-v2'); & python -m unittest discover -s scripts/tests -p 'test_verify_models.py'; break }
            'training-test' { $env:PYTHONPATH = (Join-Path $root 'training/src'); & python -m unittest discover -s training/tests; break }
            default {
                if ($Recipe -like 'train-*') {
                    $script = $Recipe.Substring(6).Replace('-', '_')
                    & uv run --project training "training/$script.py" @RecipeArgs
                } else { throw "Unknown recipe: $Recipe" }
            }
        }
        if ($null -ne $LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "Command exited $LASTEXITCODE" }
    } *> $log
} catch {
    $_ | Out-String | Add-Content -LiteralPath $log
    $code = 1
}
$code | Set-Content -LiteralPath $exitFile
Write-Output "$Recipe exit=$code log=target/logs/$Recipe.log"
exit $code
