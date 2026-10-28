use anyhow::{bail, Context};
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        println!("hre-eval [--seed N] [--variants 1..64] [--output reports]\nhre-eval kappa LABEL_A.json LABEL_B.json\nConfig: config/behavior_rules.toml and config/fusion.toml if present; otherwise defaults.");
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "kappa") {
        if args.len() != 3 {
            bail!("usage: hre-eval kappa LABEL_A.json LABEL_B.json");
        }
        let a: Vec<bool> =
            serde_json::from_slice(&std::fs::read(&args[1]).context("read first label file")?)?;
        let b: Vec<bool> =
            serde_json::from_slice(&std::fs::read(&args[2]).context("read second label file")?)?;
        let kappa = hre_eval::metrics::kappa(&a, &b)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"n": a.len(), "cohens_kappa": kappa,
            "reason": if kappa.is_none() { Some("Chance agreement is one; kappa undefined") } else { None } }))?
        );
        return Ok(());
    }
    let mut seed = 20261028_u64;
    let mut variants = 4_usize;
    let mut output = PathBuf::from("reports");
    let mut arguments = args.iter();
    while let Some(option) = arguments.next() {
        let value = arguments
            .next()
            .context("each option requires a value; use --help")?;
        match option.as_str() {
            "--seed" => seed = value.parse().context("seed must be an unsigned integer")?,
            "--variants" => variants = value.parse().context("variants must be 1..64")?,
            "--output" => output = PathBuf::from(value),
            other => bail!("unknown option {other}; use --help"),
        }
    }
    let behavior = match read_config(Path::new("config/behavior_rules.toml"))? {
        Some(text) => hre_behavior::BehaviorConfig::parse(&text)?,
        None => hre_behavior::BehaviorConfig::default(),
    };
    let fusion = match read_config(Path::new("config/fusion.toml"))? {
        Some(text) => hre_fusion::FusionConfig::parse(&text)?,
        None => hre_fusion::FusionConfig::default(),
    };
    let report = hre_eval::evaluate(seed, variants, behavior, fusion)?;
    hre_eval::write_report(&report, &output)?;
    println!(
        "Synthetic engine evaluation: {} conversations, {} text cases; reports in {}",
        report.conversations.len(),
        report.text_cases.len(),
        output.display()
    );
    Ok(())
}

fn read_config(path: &Path) -> anyhow::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}
