$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$modelsDir = Join-Path $root 'models'
$manifestPath = Join-Path $modelsDir 'manifest.json'
$expected = [ordered]@{
    'nsfw.onnx' = 'bdaccd5a615761cc67f60465bc52deaa8b5298c1ba8ce43ed14d1e7a1967cd67'
    'deepfake.onnx' = 'dfa99abbee32433d8b67fbd2f9be91323f8b6e5c8e984ef70064025b65b09ac7'
    'ocr_recognizer.onnx' = 'b7945055470411e7136c07b2ce74e4a46581723bafba44433cc01fc560f07002'
    'ocr_detector.onnx' = '30a86f5731181461d08021402766601e4302a9b9b9666be8aff402696339cdff'
    'ocr_vocab.json' = '1b4284d1c01959346570b7dccfd6660ebcafc1f2c7edacf4d0b14b48bb2f2348'
    'whisper_encoder.onnx' = '6642befb640f950d4a8cbbd17834d59e7e75f575b81ccf213e06b050623ab1dd'
    'whisper_decoder.onnx' = 'ab79e3f2a9a3d98f159f853a3172120a38af7eb5f7863d706aa7d39c228f009e'
    'whisper_tokenizer.json' = '27fc476bfe7f17299480be2273fc0608e4d5a99aba2ab5dec5374b4482d1a566'
    'whisper_preprocessor_config.json' = 'a6a76d28c93edb273669eb9e0b0636a2bddbb1272c3261e47b7ca6dfdbac1b8d'
}
foreach ($name in $expected.Keys) {
    $path = Join-Path $modelsDir $name
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing artifact: $name" }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $expected[$name]) {
        throw "Artifact hash mismatch: $name"
    }
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
foreach ($task in @('nsfw', 'deepfake')) {
    $entry = @($manifest.models | Where-Object { $_.task -eq $task })
    $filename = "$task.onnx"
    if ($entry.Count -ne 1 -or $entry[0].path -ne $filename) { throw "Expected one $task entry with path $filename" }
    $entry[0].sha256 = $expected[$filename]
    $entry[0].version = 'transferred-1'
    $entry[0].license = 'Apache-2.0'
    if ($task -eq 'nsfw') { $entry[0].labels = @('normal', 'nsfw') }
}
$ocr = @($manifest.models | Where-Object { $_.task -eq 'ocr' })
if ($ocr.Count -ne 1 -or $ocr[0].path -ne 'ocr_recognizer.onnx') { throw 'Expected one OCR recognizer entry' }
$ocr[0].sha256 = $expected['ocr_recognizer.onnx']
$ocr[0].version = 'transferred-1'
$whisper = @($manifest.models | Where-Object { $_.task -eq 'whisper' })
if ($whisper.Count -gt 1) { throw 'Duplicate whisper task entries' }
if ($whisper.Count -eq 1 -and ($whisper[0].path -ne 'whisper_encoder.onnx' -or $whisper[0].sha256 -ne $expected['whisper_encoder.onnx'])) {
    throw 'Existing Whisper entry differs; preserve it for review'
}
if ($whisper.Count -eq 0) {
    $manifest.models += [PSCustomObject]@{
        id = 'whisper'; version = 'onnx-tiny-1'; task = 'whisper'; backend = 'onnx'
        path = 'whisper_encoder.onnx'; sha256 = $expected['whisper_encoder.onnx']
        tokenizer = 'whisper_tokenizer.json'; labels = @(); thresholds = [PSCustomObject]@{}
        license = 'Apache-2.0'; tier = 'default'; status = 'built'
        outputs = @([PSCustomObject]@{ name = 'last_hidden_state'; dtype = 'float32'; shape = @(1, 1500, 384) })
    }
}
$backup = Join-Path $modelsDir ("manifest.before-known-models." + [guid]::NewGuid().ToString('N') + '.json')
$temporary = Join-Path $modelsDir ("manifest.pending." + [guid]::NewGuid().ToString('N') + '.json')
$json = $manifest | ConvertTo-Json -Depth 50
try {
    [System.IO.File]::WriteAllText($temporary, $json + "`n", (New-Object System.Text.UTF8Encoding $false))
    [System.IO.File]::Replace($temporary, $manifestPath, $backup)
} finally {
    if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary }
}
Write-Output "Admitted hash-matched OCR and Whisper metadata; backup: $backup"
Write-Output 'The reported NSFW and deepfake hashes matched; runtime inference still needs verification.'
