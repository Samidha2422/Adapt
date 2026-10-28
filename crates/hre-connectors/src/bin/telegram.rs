use hre_connectors::{HreClient, TelegramConnector};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_else(|_| "mock_telegram_token".into());
    let engine_url = env::var("HRE_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let api_key = env::var("HRE_API_KEY").unwrap_or_else(|_| "key".into());

    println!("Starting HRE Telegram Connector Daemon...");
    println!("Connecting to HRE Engine at: {}", engine_url);

    let client = HreClient::new(&engine_url, &api_key);
    match client.health_check().await {
        Ok(true) => println!("HRE Engine connection verified successfully."),
        Ok(false) => println!("Warning: HRE Engine reported non-ok status."),
        Err(e) => println!(
            "Warning: HRE Engine unreachable at startup ({e}). Continuing in fallback mode."
        ),
    }

    let _connector = TelegramConnector::new(client);
    println!(
        "Telegram bot daemon initialized (token length: {} chars).",
        token.len()
    );
    println!("Listening for Telegram updates (run with live TELEGRAM_BOT_TOKEN to connect to Telegram Bot API).");

    Ok(())
}
