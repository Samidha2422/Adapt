use hre_core::EngineConfig;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help") {
        println!("hre-server [--mock] [--config PATH]");
        return Ok(());
    }
    let mock = args.iter().any(|arg| arg == "--mock");
    let path = args
        .windows(2)
        .find(|a| a[0] == "--config")
        .map_or("config/engine.toml", |a| a[1].as_str());
    let config = EngineConfig::load(Path::new(path))?;
    config.validate()?;
    let bind = config.bind;

    println!("============================================================");
    println!("  Adapt Harassment Risk Engine (hre-server v0.1.0)");
    println!("============================================================");
    println!("Binding address: http://{bind}");
    if mock {
        println!("Loading mode: Mock / Simulated (instant startup)");
    } else {
        println!("Loading mode: Real ONNX Multimodal Pipeline");
        println!("  -> Allocating & verifying ~2.1 GB of neural models...");
        println!("  -> Text: 11-label XLM-RoBERTa (1.11 GB)");
        println!("  -> Vision: NSFW ViT (344 MB) + Deepfake ViT (343 MB)");
        println!("  -> Audio: Whisper-tiny.en Seq2Seq (151 MB)");
        println!("  -> OCR: DBNet Detector + PP-OCRv4 Recognizer (13 MB)");
        println!("  -> Behavior: 47-feature Message GBDT + 8-feature Call GBDT");
        println!("Please wait ~25-35s for weights to load and warm up...");
        println!("(Tip: Use --mock for instant startup during UI/dashboard development)");
    }
    println!("------------------------------------------------------------");

    let app = hre_api::router(config, mock).await?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, mock, "listening");

    println!("✓ All models verified and loaded successfully!");
    println!("✓ Listening on http://{bind}");
    println!("✓ Moderator Dashboard: http://{bind}/dashboard");
    println!("✓ Dashboard Sign-in:   http://{bind}/dashboard/login");
    println!("============================================================");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
