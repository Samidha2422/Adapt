param([string] $BaseUrl = 'http://127.0.0.1:8080')
$ErrorActionPreference = 'Stop'
$key = ($env:HRE_API_KEYS -split ',')[0]
if (-not $key) { throw 'Export HRE_API_KEYS before seeding' }
$headers = @{ 'X-API-Key' = $key }
$run = [guid]::NewGuid().ToString('N')
$start = [datetime]::UtcNow.AddMinutes(-30)
function Send-Event([string] $Sender, [string] $Recipient, [int] $Offset, [bool] $Blocked = $false, [bool] $Call = $false, [string] $ImageCaption = $null) {
    $body = @{
        event_id = [guid]::NewGuid().ToString()
        kind = 'message'
        occurred_at = $start.AddSeconds($Offset).ToString('o')
        sender_id = $Sender
        recipient_id = $Recipient
        platform = 'demo-chat'
        channel = @{ type = 'dm'; conversation_id = "seed-$run" }
        context = @{ is_contact = $false; recipient_blocked_sender = $Blocked }
        message = @{ text = 'Hello, how is your day?' }
    }
    if ($Call) {
        $body.Remove('message')
        $body.kind = 'call_event'
        $body.channel.type = 'call'
        $body.call = @{ direction = 'outgoing'; answered = $true; duration_s = 900; voicemail = $false }
        $body.context.is_contact = $true
    }
    if ($ImageCaption) {
        $body.Remove('message')
        $body.kind = 'image'
        $body.media = @{ mime = 'image/png'; caption = $ImageCaption }
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

Write-Output 'Real scenarios 1, 2, 3, 4, 7 checked on synthetic inputs. Scenarios 5-6,8-9 require teammate integration.'
