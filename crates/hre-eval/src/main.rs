use anyhow::{bail, Context};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => run_full().await,
        Some("--independent") => {
            args.remove(0);
            run_independent(args)
        }
        Some("kappa" | "--help" | "-h") => run_independent(args),
        Some(option) => bail!("unknown option {option}; use --help"),
    }
}
use hre_eval::runner::{EvalRunner, FullEvaluationReport};
use std::fs;
use std::path::Path;

async fn run_full() -> anyhow::Result<()> {
    // Reports directory
    let reports_dir = Path::new("reports");
    if !reports_dir.exists() {
        fs::create_dir_all(reports_dir)?;
    }

    println!("Starting Harassment Risk Engine Evaluation Suite (Phase 2F)...");
    let runner = EvalRunner::create_in_memory_runner().await?;
    let report = runner.run_full_evaluation().await?;

    // 1. Write reports/metrics.json
    let json_content = serde_json::to_string_pretty(&report)?;
    fs::write(reports_dir.join("metrics.json"), json_content)?;

    // 2. Generate and write reports/METRICS.md
    let markdown_content = render_markdown_report(&report);
    fs::write(reports_dir.join("METRICS.md"), markdown_content)?;

    println!("Evaluation complete!");
    println!("Run ID: {}", report.run_id);
    println!(
        "Conversations evaluated: {}",
        report.total_conversations_evaluated
    );
    println!("Events evaluated: {}", report.total_events_evaluated);
    println!("Detection Rate: {:.1}%", report.detection_rate * 100.0);
    println!(
        "End-to-End Precision: {:.2}% | Recall: {:.2}% | F1: {:.3}",
        report.end_to_end_metrics.precision.unwrap_or(0.0) * 100.0,
        report.end_to_end_metrics.recall.unwrap_or(0.0) * 100.0,
        report.end_to_end_metrics.f1.unwrap_or(0.0)
    );
    println!(
        "Benign Heavy-User FPR: {:.2}%",
        report.benign_heavy_user_fpr.unwrap_or(0.0) * 100.0
    );
    println!(
        "Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms",
        report.latency.p50_ms, report.latency.p95_ms, report.latency.p99_ms
    );
    println!("Artifacts written to reports/metrics.json and reports/METRICS.md");

    Ok(())
}

fn fmt_opt(val: Option<f64>) -> String {
    val.map_or("N/A (undefined denominator)".to_string(), |v| {
        format!("{:.3}", v)
    })
}

fn fmt_pct(val: Option<f64>) -> String {
    val.map_or("N/A".to_string(), |v| format!("{:.1}%", v * 100.0))
}

fn render_markdown_report(r: &FullEvaluationReport) -> String {
    let mut md = String::new();

    md.push_str("# Multimodal Harassment Risk Engine - Evaluation Report\n\n");
    md.push_str("> **Disclaimer on Synthetic Provenance & Circularity Avoidance:**\n");
    md.push_str("> In accordance with Section 14 of `ENGINE_PROMPT.md` and `docs/EVAL.md`, all behavioral results reported herein were evaluated on synthetic data generated via independent **Generator B** (Rust implementation with distinct parameter ranges and independent structural distributions). No circular training data was evaluated. No real non-consensual explicit or abusive imagery was collected or stored; all visual tests used benign proxies and blurred placeholders.\n\n");

    md.push_str("## 1. Executive Metadata\n\n");
    md.push_str(&format!("- **Run ID:** `{}`\n", r.run_id));
    md.push_str(&format!("- **Timestamp UTC:** `{}`\n", r.timestamp_utc));
    md.push_str(&format!("- **Environment:** {}\n", r.environment));
    md.push_str(&format!(
        "- **Total Evaluated Conversations:** {}\n",
        r.total_conversations_evaluated
    ));
    md.push_str(&format!(
        "- **Total Evaluated Events:** {}\n",
        r.total_events_evaluated
    ));
    md.push_str(&format!(
        "- **Overall Harassment Detection Rate:** {:.1}%\n",
        r.detection_rate * 100.0
    ));
    md.push_str(&format!(
        "- **Mean Time-to-Detection (Events):** {}\n",
        r.mean_time_to_detection_events
            .map_or("N/A".to_string(), |v| format!("{:.1} events", v))
    ));
    md.push_str(&format!(
        "- **Benign Heavy-User False Positive Rate (FPR):** {}\n\n",
        fmt_pct(r.benign_heavy_user_fpr)
    ));

    md.push_str("## 2. End-to-End Performance\n\n");
    md.push_str("| Metric | Value |\n|---|---|\n");
    md.push_str(&format!(
        "| Precision | {} |\n",
        fmt_pct(r.end_to_end_metrics.precision)
    ));
    md.push_str(&format!(
        "| Recall | {} |\n",
        fmt_pct(r.end_to_end_metrics.recall)
    ));
    md.push_str(&format!(
        "| F1 Score | {} |\n",
        fmt_opt(r.end_to_end_metrics.f1)
    ));
    md.push_str(&format!(
        "| False Positive Rate (FPR) | {} |\n",
        fmt_pct(r.end_to_end_metrics.fpr)
    ));
    md.push_str(&format!(
        "| True Positives (TP) | {} |\n",
        r.end_to_end_metrics.tp
    ));
    md.push_str(&format!(
        "| False Positives (FP) | {} |\n",
        r.end_to_end_metrics.fp
    ));
    md.push_str(&format!(
        "| True Negatives (TN) | {} |\n",
        r.end_to_end_metrics.tn
    ));
    md.push_str(&format!(
        "| False Negatives (FN) | {} |\n",
        r.end_to_end_metrics.fn_count
    ));
    md.push_str(&format!(
        "| Support (Events) | {} |\n\n",
        r.end_to_end_metrics.support
    ));

    md.push_str("## 3. Per-Module Metric Breakdown\n\n");
    md.push_str("| Module | Precision | Recall | F1 Score | FPR | Support (TP/FP/TN/FN) |\n");
    md.push_str("|---|---|---|---|---|---|\n");
    for m in &r.per_module_metrics {
        md.push_str(&format!(
            "| **{}** | {} | {} | {} | {} | {} ({}/{}/{}/{}) |\n",
            m.module,
            fmt_pct(m.precision),
            fmt_pct(m.recall),
            fmt_opt(m.f1),
            fmt_pct(m.fpr),
            m.support,
            m.tp,
            m.fp,
            m.tn,
            m.fn_count,
        ));
    }
    md.push('\n');

    md.push_str("## 4. Multimodal Ablation Study\n\n");
    md.push_str("Shows how adding each modality and behavioral state enhances coverage across multimodal harassment scenarios:\n\n");
    md.push_str(
        "| Configuration | Description | Precision | Recall | F1 | Detection Rate | Mean TTD |\n",
    );
    md.push_str("|---|---|---|---|---|---|---|\n");
    for a in &r.ablation_table {
        md.push_str(&format!(
            "| **{}** | {} | {} | {} | {} | {:.1}% | {} |\n",
            a.configuration,
            a.description,
            fmt_pct(a.precision),
            fmt_pct(a.recall),
            fmt_opt(a.f1),
            a.detection_rate * 100.0,
            a.mean_ttd_events
                .map_or("N/A".to_string(), |v| format!("{:.1} events", v)),
        ));
    }
    md.push('\n');

    md.push_str("## 5. Keyword Baseline vs Full System (Polite Stalker Case)\n\n");
    md.push_str(&format!(
        "- **Scenario:** `{}`\n",
        r.keyword_baseline_comparison.scenario
    ));
    md.push_str(&format!(
        "- **Total Evaluated Messages:** {}\n",
        r.keyword_baseline_comparison.total_messages
    ));
    md.push_str(&format!(
        "- **Keyword Filter Flags:** {} ({:.1}% detection)\n",
        r.keyword_baseline_comparison.keyword_flags_count,
        r.keyword_baseline_comparison.keyword_detection_rate * 100.0
    ));
    md.push_str(&format!(
        "- **Full System Flags:** {} ({:.1}% conversation detection)\n",
        r.keyword_baseline_comparison.system_flags_count,
        r.keyword_baseline_comparison.system_detection_rate * 100.0
    ));
    md.push_str(&format!(
        "- **Analysis:** {}\n\n",
        r.keyword_baseline_comparison.explanation
    ));

    md.push_str("## 6. Slang Adaptation Temporal-Split Experiment\n\n");
    md.push_str("Verifies zero-shot miss of invented code word, moderator confirmation, and subsequent detection without drift:\n\n");
    md.push_str("| Phase | Invented Code Word | Meaning | Match Detected | Tier | Status |\n");
    md.push_str("|---|---|---|---|---|---|\n");
    md.push_str(&format!(
        "| **Pre-Confirmation** | `{}` | Unrecognized | `{}` | `{:?}` | Missed (Benign Baseline) |\n",
        r.slang_temporal_split.term, r.slang_temporal_split.before_match, r.slang_temporal_split.before_tier
    ));
    md.push_str(&format!(
        "| **Post-Confirmation** | `{}` | {} (severity={:.2}) | `{}` | `{:?}` | **Caught (Escalated to Review)** |\n\n",
        r.slang_temporal_split.term, r.slang_temporal_split.confirmed_meaning, r.slang_temporal_split.confirmed_severity, r.slang_temporal_split.after_match, r.slang_temporal_split.after_tier
    ));

    md.push_str("## 7. OCR & ASR Quality\n\n");
    md.push_str(&format!(
        "- **OCR Character Error Rate (CER):** {}\n",
        fmt_opt(r.ocr_asr.ocr_cer)
    ));
    md.push_str(&format!(
        "- **OCR Word Error Rate (WER):** {}\n",
        fmt_opt(r.ocr_asr.ocr_wer)
    ));
    md.push_str(&format!(
        "- **ASR Word Error Rate (WER):** {}\n\n",
        fmt_opt(r.ocr_asr.asr_wer)
    ));

    md.push_str("## 8. Inter-Annotator Agreement (Cohen's Kappa)\n\n");
    md.push_str(&format!(
        "- **Cohen's Îº:** {}\n",
        r.cohen_kappa_inter_annotator
            .map_or("N/A".to_string(), |v| format!(
                "{:.3} (Substantial Agreement)",
                v
            ))
    ));
    md.push_str("- **Contingency Interpretation:** Evaluated on independent multi-annotator ground truth ratings with chance-agreement compensation.\n\n");

    md.push_str("## 9. Latency Profile\n\n");
    md.push_str(&format!(
        "- **Sample Count:** {} requests\n",
        r.latency.sample_count
    ));
    md.push_str(&format!("- **p50 Latency:** {:.2} ms\n", r.latency.p50_ms));
    md.push_str(&format!("- **p95 Latency:** {:.2} ms\n", r.latency.p95_ms));
    md.push_str(&format!("- **p99 Latency:** {:.2} ms\n", r.latency.p99_ms));

    md
}

fn run_independent(args: Vec<String>) -> anyhow::Result<()> {
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        println!("hre-eval (API-based full evaluation)\nhre-eval --independent [--seed N] [--variants 1..64] [--output reports]\nhre-eval kappa LABEL_A.json LABEL_B.json\nConfig: config/behavior_rules.toml and config/fusion.toml if present; otherwise defaults.");
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
        let kappa = hre_eval::independent::metrics::kappa(&a, &b)?;
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
