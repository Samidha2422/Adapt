use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn directory() -> std::io::Result<PathBuf> {
    let path = std::env::temp_dir().join(format!(
        "hre-eval-test-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_nanos(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path)?;
    Ok(path)
}

#[test]
fn independent_cli_writes_reports_without_models_or_training_files(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = directory()?;
    let output = Command::new(env!("CARGO_BIN_EXE_hre-eval"))
        .arg("--independent")
        .current_dir(&path)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: hre_eval::Report =
        serde_json::from_slice(&std::fs::read(path.join("reports/metrics.json"))?)?;
    assert_eq!(report.variants, 4);
    assert_eq!(report.conversations.len(), 68);
    assert_eq!(report.text_cases.len(), 18);
    let markdown = std::fs::read_to_string(path.join("reports/METRICS.md"))?;
    assert!(markdown.contains("null"));
    assert!(markdown.contains("Synthetic"));
    std::fs::remove_file(path.join("reports/metrics.json"))?;
    std::fs::remove_file(path.join("reports/METRICS.md"))?;
    std::fs::remove_dir(path.join("reports"))?;
    std::fs::remove_dir(path)?;
    Ok(())
}

#[test]
fn default_cli_reports_unavailable_audio_without_writing_incomplete_reports(
) -> Result<(), Box<dyn std::error::Error>> {
    let path = directory()?;
    let output = Command::new(env!("CARGO_BIN_EXE_hre-eval"))
        .current_dir(&path)
        .output()?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("audio analysis unavailable (HTTP 501)")
    );
    assert!(!path.join("reports/metrics.json").exists());
    assert!(!path.join("reports/METRICS.md").exists());
    std::fs::remove_dir(path.join("reports"))?;
    std::fs::remove_dir(path)?;
    Ok(())
}

#[test]
fn cli_rejects_invalid_arguments_without_writing_reports() -> Result<(), Box<dyn std::error::Error>>
{
    let path = directory()?;
    for args in [
        vec!["--variants", "0"],
        vec!["--variants"],
        vec!["--mock", "true"],
        vec!["--seed", "invalid"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_hre-eval"))
            .arg("--independent")
            .args(args)
            .current_dir(&path)
            .output()?;
        assert!(!output.status.success());
        assert!(!path.join("reports").exists());
    }
    std::fs::remove_dir(path)?;
    Ok(())
}
