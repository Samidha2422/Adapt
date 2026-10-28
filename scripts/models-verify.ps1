$ErrorActionPreference = 'Stop'
$depsClean = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/model-verify-deps-clean'
$deps = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/model-verify-deps'
$env:PYTHONPATH = if (Test-Path $depsClean) { $depsClean } else { $deps }
$python = Get-Command python -ErrorAction Stop
& $python.Source (Join-Path $PSScriptRoot 'verify-models.py') (Join-Path $PSScriptRoot '../models')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
