param([string] $BaseUrl = 'http://127.0.0.1:8080')
$ErrorActionPreference = 'Stop'
$key = ($env:HRE_API_KEYS -split ',')[0]
if (-not $key) { throw 'Export HRE_API_KEYS before seeding' }
$headers = @{ 'X-API-Key' = $key }
$run = [guid]::NewGuid().ToString('N')
$start = [datetime]::UtcNow.AddMinutes(-30)
function Send-Event([string] $Sender, [string] $Recipient, [int] $Offset, [bool] $Blocked = $false, [bool] $Call = $false, [string] $ImageCaption = $null, [string] $AudioCaption = $null, [bool] $UnansweredCall = $false, [string] $Text = 'Hello, how is your day?') {
    $body = @{
        event_id = [guid]::NewGuid().ToString()
        kind = 'message'
        occurred_at = $start.AddSeconds($Offset).ToString('o')
        sender_id = $Sender
        recipient_id = $Recipient
        platform = 'demo-chat'
        channel = @{ type = 'dm'; conversation_id = "seed-$run" }
        context = @{ is_contact = $false; recipient_blocked_sender = $Blocked }
        message = @{ text = $Text }
    }
    if ($Call) {
        $body.Remove('message')
        $body.kind = 'call_event'
        $body.channel.type = 'call'
        $body.call = @{ direction = 'outgoing'; answered = $true; duration_s = 900; voicemail = $false }
        $body.context.is_contact = $true
    }
    if ($UnansweredCall) {
        $body.Remove('message')
        $body.kind = 'call_event'
        $body.channel.type = 'call'
        $body.call = @{ direction = 'incoming'; answered = $false; duration_s = 0; voicemail = $false }
        $body.context.is_contact = $false
    }
    if ($ImageCaption) {
        $body.Remove('message')
        $body.kind = 'image'
        $body.media = @{ mime = 'image/png'; caption = $ImageCaption }
    }
    if ($AudioCaption) {
        $body.Remove('message')
        $body.kind = 'audio'
        $body.media = @{ mime = 'audio/mpeg'; caption = $AudioCaption }
    }
    Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/analyze" -Headers $headers -ContentType 'application/json' -Body ($body | ConvertTo-Json -Depth 8)
}
function Assert-Tier($Result, [string] $Expected, [string] $Name) {
    if ($Result.tier -ne $Expected) { throw "$Name expected $Expected, got $($Result.tier) severity=$($Result.severity)" }
    Write-Output "$Name PASS tier=$($Result.tier) severity=$($Result.severity) simulated=$($Result.simulated)"
}
$health = Invoke-RestMethod -Uri "$BaseUrl/v1/health"
if ($health.mock) {
    $fixtures = @(
        @('polite_stalker','review'), @('pile_on','high'), @('cyberflasher','high'),
        @('screenshot','high'), @('voice_note','urgent'), @('call_log','high'),
        @('heavy_benign','low'), @('slang','review'), @('evidence','review')
    )
    foreach ($fixture in $fixtures) {
        $result = Send-Event "mock:$($fixture[0])" "mock-target-$run" 0
        Assert-Tier $result $fixture[1] $fixture[0]
        if (-not $result.simulated) { throw 'Mock must be labelled simulated' }
    }
    Write-Output '9 mock fixture tiers checked; no real media/slang/evidence integration claimed.'
    exit 0
}
$sawReview = $false
for ($i = 0; $i -lt 80; $i++) {
    $result = Send-Event "stalker-$run" "target-$run" ($i * 10) ($i -ge 70)
    if ($result.tier -eq 'review') { $sawReview = $true }
}
if (-not $sawReview) { throw 'Scenario 1 never entered review' }
Assert-Tier $result 'high' '1 polite stalker'
for ($i = 0; $i -lt 10; $i++) { $pile = Send-Event "pile-$run-$i" "pile-target-$run" (900 + $i) }
Assert-Tier $pile 'high' '2 pile-on'
if ($pile.secondary_actions -notcontains 'collapse_replies') { throw 'Pile-on did not enable feed protection' }
for ($i = 0; $i -lt 50; $i++) {
    $a = Send-Event "couple-a-$run" "couple-b-$run" (1000 + 2 * $i)
    $b = Send-Event "couple-b-$run" "couple-a-$run" (1001 + 2 * $i)
    if ($a.tier -ne 'low' -or $b.tier -ne 'low') { throw 'Healthy messages flagged' }
}
for ($i = 0; $i -lt 5; $i++) {
    $a = Send-Event "couple-a-$run" "couple-b-$run" (1200 + 2 * $i) $false $true
    $b = Send-Event "couple-b-$run" "couple-a-$run" (1201 + 2 * $i) $false $true
    if ($a.tier -ne 'low' -or $b.tier -ne 'low') { throw 'Healthy answered calls flagged' }
}
Assert-Tier $b 'low' '7 healthy couple'

$cf = Send-Event "cyberflasher-$run" "target-$run" 1300 $false $false 'cyberflashing'
Assert-Tier $cf 'high' '3 cyberflashing'
if ($cf.recommended_action -ne 'blur_media') { throw 'Cyberflashing did not blur media' }

$meme = Send-Event "meme-$run" "target-$run" 1310 $false $false 'I will kill you and your family'
Assert-Tier $meme 'high' '4 screenshot meme'
if ($meme.recommended_action -ne 'blur_media') { throw 'Screenshot meme did not blur media' }

$vn = Send-Event "voice-$run" "target-$run" 1320 $false $false $null 'threat:I will find you and kill you'
if ($vn.tier -ne 'high' -and $vn.tier -ne 'urgent') { throw "Scenario 5 expected high or urgent, got $($vn.tier)" }
if ($vn.secondary_actions -notcontains 'show_transcript') { throw 'Scenario 5 did not show transcript' }
Write-Output "5 voice note PASS tier=$($vn.tier) severity=$($vn.severity) simulated=$($vn.simulated)"

for ($i = 0; $i -lt 35; $i++) {
    $callRes = Send-Event "caller-$run" "target-$run" (1330 + $i * 60) ($i -ge 30) $false $null $null $true
}
if ($callRes.tier -ne 'high' -and $callRes.tier -ne 'urgent') { throw "Scenario 6 expected high or urgent, got $($callRes.tier)" }
Write-Output "6 call log PASS tier=$($callRes.tier) severity=$($callRes.severity) simulated=$($callRes.simulated)"

# Scenario 8: Slang adaptation
$modToken = ($env:HRE_MODERATOR_TOKENS -split ',')[0]
if (-not $modToken) { $modToken = 'mod' }
$slangBefore = Send-Event "slang-user-$run" "target-$run" 1340 $false $false $null $null $false "You are such a vexling"
if ($slangBefore.tier -ne 'low') { throw "Scenario 8 expected low before confirmation, got $($slangBefore.tier)" }

$confBody = @{ term = "vexling"; meaning = "invented harassing slur"; severity = 0.45 }
$null = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/slang/terms" -Headers @{ Authorization = "Bearer $modToken" } -ContentType 'application/json' -Body ($confBody | ConvertTo-Json)

$slangAfter = Send-Event "slang-user-$run" "target-$run" 1345 $false $false $null $null $false "Stop being a vexling"
Assert-Tier $slangAfter 'review' '8 slang update'

# Scenario 9: Evidence export & cryptographic verification
$expReq = @{ subject_id = "target-$run" }
$expRes = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/evidence/export" -Headers $headers -ContentType 'application/json' -Body ($expReq | ConvertTo-Json)
if (-not $expRes.manifest_hash -or -not $expRes.hmac_signature) { throw 'Scenario 9 export missing manifest or HMAC signature' }
$verifyRes = Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/evidence/verify" -Headers $headers -ContentType 'application/json' -Body ($expRes | ConvertTo-Json -Depth 10)
if (-not $verifyRes.valid) { throw 'Scenario 9 cryptographic verification failed' }
Write-Output "9 evidence export PASS records=$($expRes.record_count) manifest=$($expRes.manifest_hash.Substring(0,12))... verified=true"

Write-Output 'All 9 real scenarios checked on synthetic inputs with full cryptographic and policy verification.'
