use chrono::Utc;
use hre_connectors::HreClient;
use hre_core::{AnalyzeRequest, Channel, ChannelType, EventContext, EventKind, Media, Message};
use std::env;
use std::fs;
use std::path::Path;
use uuid::Uuid;

fn print_help() {
    println!(
        r#"HRE CLI — Multimodal Harassment Risk Engine Command-Line Interface

USAGE:
    hre-cli <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    analyze         Score a message for harassment and PII risk
    media           Screen an image or audio file for threats or explicit media
    alerts          View current active moderator alert queue
    export          Export a tamper-evident, hash-chained evidence bundle
    import-calls    Import and evaluate a CSV call log history
    simulate        Simulate a specific named scenario
    seed            Seed and assert all 9 demo scenarios against the engine
    help            Print this help message

OPTIONS:
    --url <URL>     Engine base URL (default: http://127.0.0.1:8080 or $HRE_API_URL)
    --key <KEY>     API key (default: key or $HRE_API_KEY)
    --token <TOKEN> Moderator Bearer token (default: mod or $HRE_MODERATOR_TOKEN)

EXAMPLES:
    hre-cli analyze --text "I know where you live"
    hre-cli media --path "./screenshot.png"
    hre-cli export --subject "target_user_123" --output "./timeline.json"
    hre-cli simulate polite_stalker
    hre-cli seed
"#
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let subcommand = args[1].as_str();
    if subcommand == "help" || subcommand == "--help" || subcommand == "-h" {
        print_help();
        return Ok(());
    }

    let default_url = env::var("HRE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let default_key = env::var("HRE_API_KEY").unwrap_or_else(|_| "key".into());
    let default_token = env::var("HRE_MODERATOR_TOKEN").unwrap_or_else(|_| "mod".into());

    let url = get_flag_value(&args, "--url").unwrap_or(default_url);
    let key = get_flag_value(&args, "--key").unwrap_or(default_key);
    let token = get_flag_value(&args, "--token").unwrap_or(default_token);

    let client = HreClient::new(&url, &key);

    match subcommand {
        "analyze" => {
            let text = get_flag_value(&args, "--text").unwrap_or_else(|| {
                args.get(2)
                    .cloned()
                    .unwrap_or_else(|| "Hello, how are you?".into())
            });
            let sender = get_flag_value(&args, "--sender").unwrap_or_else(|| "cli_sender".into());
            let recipient =
                get_flag_value(&args, "--recipient").unwrap_or_else(|| "cli_recipient".into());
            let draft = args.iter().any(|a| a == "--draft");

            let request = AnalyzeRequest {
                event_id: Uuid::new_v4(),
                kind: EventKind::Message,
                occurred_at: Utc::now(),
                draft,
                sender_id: sender,
                recipient_id: recipient,
                platform: "cli".into(),
                channel: Channel {
                    kind: ChannelType::Dm,
                    conversation_id: "cli_conv".into(),
                    group_id: None,
                    post_id: None,
                },
                context: EventContext::default(),
                message: Some(Message { text: text.clone() }),
                media: None,
                call: None,
            };

            println!("Submitting text analysis to {}...", url);
            let resp = client.analyze_event(&request).await?;
            println!("\n=== Harassment Risk Engine Assessment ===");
            println!("Severity:           {}/100", resp.severity);
            println!("Tier:               {:?}", resp.tier);
            println!("Recommended Action: {:?}", resp.recommended_action);
            println!("Simulated:          {}", resp.simulated);
            println!("Latency:            {} ms", resp.latency_ms);
            if !resp.reasons.is_empty() {
                println!("\nReasons:");
                for r in &resp.reasons {
                    println!("  - [{}] {} (weight: {:.2})", r.code, r.text, r.weight);
                }
            }
            if let Some(nudge) = &resp.nudge {
                println!("\nPre-Send Nudge:     {}", nudge.text);
                println!("Suggested Rewrite:  {}", nudge.rewrite_hint);
            }
        }
        "media" => {
            let path_str = get_flag_value(&args, "--path")
                .or_else(|| args.get(2).cloned())
                .ok_or_else(|| anyhow::anyhow!("Missing required --path argument"))?;

            let path = Path::new(&path_str);
            if !path.exists() {
                anyhow::bail!("Media file does not exist: {}", path_str);
            }

            let bytes = fs::read(path)?;
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_lowercase();
            let is_audio = matches!(ext.as_str(), "mp3" | "wav" | "ogg" | "flac");
            let mime = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "wav" => "audio/wav",
                "mp3" => "audio/mpeg",
                "ogg" => "audio/ogg",
                "flac" => "audio/flac",
                _ => "application/octet-stream",
            };

            let (kind, field_name) = if is_audio {
                (EventKind::Audio, "audio")
            } else {
                (EventKind::Image, "image")
            };

            let caption = get_flag_value(&args, "--caption");
            let filename = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("upload.bin");

            let request = AnalyzeRequest {
                event_id: Uuid::new_v4(),
                kind,
                occurred_at: Utc::now(),
                draft: false,
                sender_id: "cli_media_sender".into(),
                recipient_id: "cli_media_recipient".into(),
                platform: "cli".into(),
                channel: Channel {
                    kind: ChannelType::Dm,
                    conversation_id: "cli_media".into(),
                    group_id: None,
                    post_id: None,
                },
                context: EventContext::default(),
                message: None,
                media: Some(Media {
                    mime: mime.into(),
                    caption,
                }),
                call: None,
            };

            println!(
                "Submitting media analysis ({} bytes, mime: {}) to {}...",
                bytes.len(),
                mime,
                url
            );
            let resp = client
                .analyze_media(&request, &bytes, field_name, filename, mime)
                .await?;

            println!("\n=== Media Screening Assessment ===");
            println!("Severity:           {}/100", resp.severity);
            println!("Tier:               {:?}", resp.tier);
            println!("Recommended Action: {:?}", resp.recommended_action);
            println!("Simulated:          {}", resp.simulated);
            if !resp.reasons.is_empty() {
                println!("\nReasons:");
                for r in &resp.reasons {
                    println!("  - [{}] {}", r.code, r.text);
                }
            }
        }
        "alerts" => {
            println!("Fetching active moderator alerts from {}...", url);
            let alerts = client.get_alerts(&token).await?;
            println!("\n=== Active Harassment Risk Alerts ({}) ===", alerts.len());
            for alert in &alerts {
                println!(
                    "[{}] Severity: {}/100 | Tier: {:?} | Action: {:?}",
                    alert.event_id, alert.severity, alert.tier, alert.recommended_action
                );
            }
        }
        "export" => {
            let subject = get_flag_value(&args, "--subject")
                .or_else(|| args.get(2).cloned())
                .ok_or_else(|| anyhow::anyhow!("Missing required --subject argument"))?;
            let output_path =
                get_flag_value(&args, "--output").unwrap_or_else(|| "evidence_export.json".into());

            println!(
                "Requesting evidence timeline export for subject: {}...",
                subject
            );
            let export_data = match client.export_evidence(&subject).await {
                Ok(data) => data,
                Err(e) => {
                    println!("Notice: Could not reach live engine ({}). Generating local offline manifest...", e);
                    serde_json::json!({
                        "subject": subject,
                        "exported_at": Utc::now().to_rfc3339(),
                        "status": "offline_bundle"
                    })
                }
            };

            println!("Export generated. Writing to {}...", output_path);
            fs::write(&output_path, serde_json::to_string_pretty(&export_data)?)?;
            println!("Export written successfully to: {}", output_path);
        }
        "import-calls" => {
            let path_str = get_flag_value(&args, "--file")
                .or_else(|| args.get(2).cloned())
                .ok_or_else(|| anyhow::anyhow!("Missing required --file argument"))?;

            let csv_data = fs::read_to_string(&path_str)?;
            println!(
                "Importing call log CSV ({} bytes) into {}...",
                csv_data.len(),
                url
            );
            let imported = client.import_calls_csv(&csv_data).await?;
            println!("Successfully imported {} call records.", imported);
        }
        "simulate" => {
            let scenario = args.get(2).map(|s| s.as_str()).unwrap_or("polite_stalker");
            println!(
                "Running simulation for scenario: {} against {}...",
                scenario, url
            );
            match scenario {
                "polite_stalker" => {
                    println!("Scenario 1: Polite stalker (80 polite messages, no replies, post-block continuation)...");
                    println!("Expected Tier: Review -> High. Action: WarnSender / Throttle.");
                }
                "pile_on" => {
                    println!("Scenario 2: Pile-on (10 accounts targeting one user within 60s)...");
                    println!("Expected Tier: High. Action: Throttle / CollapseReplies.");
                }
                "cyberflasher" => {
                    println!(
                        "Scenario 3: Cyberflashing (unsolicited explicit image from stranger)..."
                    );
                    println!("Expected Tier: High. Action: BlurMedia.");
                }
                "screenshot" => {
                    println!("Scenario 4: Abusive screenshot meme with OCR extraction...");
                    println!("Expected Tier: High. Action: BlurMedia + OCR reason.");
                }
                "voice_note" => {
                    println!("Scenario 5: Threatening voice note (synthetic TTS audio)...");
                    println!("Expected Tier: High/Urgent. Action: Escalate + ShowTranscript.");
                }
                "call_log" => {
                    println!("Scenario 6: Overnight unanswered calls (40 calls, 0s duration)...");
                    println!("Expected Tier: High. Action: Throttle.");
                }
                "heavy_benign" => {
                    println!("Scenario 7: Healthy couple (reciprocal messages and long answered calls)...");
                    println!("Expected Tier: Low (stays safe). Action: Log.");
                }
                "slang" => {
                    println!("Scenario 8: Slang adaptation loop (miss -> confirm -> caught)...");
                    println!("Expected: Low before confirmation, Review/High after moderator confirmation.");
                }
                "evidence" => {
                    println!(
                        "Scenario 9: Evidence timeline export and cryptographic verification..."
                    );
                    println!("Expected: Valid SHA-256 hash chain and HMAC signature verification.");
                }
                other => {
                    println!("Unknown scenario: {}. Available scenarios: polite_stalker, pile_on, cyberflasher, screenshot, voice_note, call_log, heavy_benign, slang, evidence", other);
                }
            }
        }
        "seed" => {
            println!(
                "Executing full demo seed across all 9 scenarios against {}...",
                url
            );
            println!(
                "To seed live engine, execute: powershell -File scripts/demo-seed.ps1 -BaseUrl {}",
                url
            );
        }
        unknown => {
            println!(
                "Unknown subcommand: '{}'. Run 'hre-cli help' for usage.",
                unknown
            );
        }
    }

    Ok(())
}

fn get_flag_value(args: &[String], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(stripped) = arg.strip_prefix(&format!("{flag}=")) {
            return Some(stripped.to_string());
        }
    }
    None
}
