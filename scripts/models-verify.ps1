$ErrorActionPreference = 'Stop'
$env:PYTHONPATH = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/model-verify-deps'
$python = Get-Command python -ErrorAction Stop
& $python.Source (Join-Path $PSScriptRoot 'verify-models.py') (Join-Path $PSScriptRoot '../models')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
