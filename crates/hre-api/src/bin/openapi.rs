use std::path::Path;

fn main() -> anyhow::Result<()> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/openapi.json".into());
    let path = Path::new(&output);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, hre_api::openapi().to_pretty_json()?)?;
    Ok(())
}
