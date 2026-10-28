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
        if ($Recipe -in @('bootstrap','fmt','format','lint','test','build','lock','run','mock','openapi','eval','bench')) { Get-Command cargo -ErrorAction Stop | Out-Null }
        if ($Recipe -like 'train-*') { Get-Command uv -ErrorAction Stop | Out-Null }
        switch ($Recipe) {
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
            'models-verify-install' { & python -m pip install --only-binary=:all: --target target/model-verify-deps onnx==1.20.1; break }
            'models-verify-test' { $env:PYTHONPATH = (Join-Path $root 'target/model-verify-deps'); & python -m unittest discover -s scripts/tests -p 'test_verify_models.py'; break }
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
