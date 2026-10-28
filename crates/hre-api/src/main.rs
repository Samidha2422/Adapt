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
    let app = hre_api::router(config, mock).await?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, mock, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
