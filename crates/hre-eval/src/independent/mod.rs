//! Engine-only evaluation on independent synthetic schedules and fresh authored text.
//! See docs/EVALUATION_SCHEMA.md for units, thresholds, limitations and null semantics.
mod generator_b;
pub mod metrics;

use hre_core::{Backend, Status};
use hre_fusion::{FusionConfig, FusionInput};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path, time::Instant};

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error(transparent)]
    Core(#[from] hre_core::CoreError),
    #[error(transparent)]
    Text(#[from] hre_text::TextError),
    #[error(transparent)]
    Behavior(#[from] hre_behavior::BehaviorError),
    #[error(transparent)]
    Fusion(#[from] hre_fusion::FusionError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(
        "label files must be nonempty JSON boolean arrays of equal length in identical item order"
    )]
    InvalidLabels,
    #[error("variants must be between 1 and 64")]
    InvalidVariants,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Measurement {
    pub status: Status,
    pub backend: Backend,
    pub simulated: bool,
    pub unit: String,
    pub threshold: Option<f64>,
    pub metrics: Option<metrics::Metrics>,
    pub pr_curve: Option<Vec<metrics::PrPoint>>,
    pub reason: Option<String>,
}

fn measured(rows: &[(bool, f64)], threshold: f64, unit: &str) -> Measurement {
    Measurement {
        status: Status::Built,
        backend: Backend::Heuristic,
        simulated: true,
        unit: unit.into(),
        threshold: Some(threshold),
        metrics: Some(metrics::Metrics::calculate(rows, threshold)),
        pr_curve: Some(metrics::pr_curve(rows)),
        reason: None,
    }
}

fn unavailable(reason: &str) -> Measurement {
    Measurement {
        status: Status::Roadmap,
        backend: Backend::Disabled,
        simulated: true,
        unit: "not_evaluated".into(),
        threshold: None,
        metrics: None,
        pr_curve: None,
        reason: Some(reason.into()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Detection {
    pub positive_conversations: usize,
    pub detected_conversations: usize,
    pub detection_rate: Option<f64>,
    pub time_to_detection_events_p50: Option<f64>,
    pub time_to_detection_events_p95: Option<f64>,
    pub missed_conversations: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResult {
    pub id: String,
    pub class: String,
    pub positive: bool,
    pub heavy_benign: bool,
    pub calls: bool,
    pub contact_events: usize,
    pub max_severity: u8,
    pub first_review_event: Option<usize>,
    pub behavior_max: f64,
    pub text_only_max_severity: u8,
    pub keyword_flagged: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextResult {
    pub id: String,
    pub language: String,
    pub positive: bool,
    pub score: f64,
    pub keyword_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Latency {
    pub n: usize,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub simulated: bool,
    pub seed: u64,
    pub variants: usize,
    pub generator: String,
    pub limitations: Vec<String>,
    pub behavior_config: hre_behavior::BehaviorConfig,
    pub fusion_config: FusionConfig,
    pub modules: BTreeMap<String, Measurement>,
    pub ablations: BTreeMap<String, Measurement>,
    pub slices: BTreeMap<String, Measurement>,
    pub detection: Detection,
    pub latency: Latency,
    pub conversations: Vec<ConversationResult>,
    pub text_cases: Vec<TextResult>,
    pub ocr_cer: Option<f64>,
    pub ocr_wer: Option<f64>,
    pub asr_wer: Option<f64>,
    pub cohens_kappa: Option<f64>,
    pub slang_temporal_split: Option<Vec<Measurement>>,
    pub unavailable_reasons: BTreeMap<String, String>,
}

fn keyword_score(text: &str) -> f64 {
    let lower = text.to_ascii_lowercase();
    f64::from(u8::from(
        ["kill", "hurt", "idiot", "worthless", "nudes", "leak"]
            .iter()
            .any(|w| lower.contains(w)),
    ))
}

/// Execute real heuristic modules without model files, persistence, API clients or C8.
pub fn evaluate(
    seed: u64,
    variants: usize,
    behavior_config: hre_behavior::BehaviorConfig,
    fusion_config: FusionConfig,
) -> Result<Report, EvalError> {
    if !(1..=64).contains(&variants) {
        return Err(EvalError::InvalidVariants);
    }
    behavior_config.validate()?;
    fusion_config.validate()?;
    let mut text_results = Vec::new();
    for case in generator_b::text_cases() {
        let report = hre_text::analyze(case.text)?;
        text_results.push(TextResult {
            id: case.id.into(),
            language: case.language.into(),
            positive: case.positive,
            score: report.primary_signal.score,
            keyword_score: keyword_score(case.text),
        });
    }
    let mut conversations = Vec::new();
    let mut latencies = Vec::new();
    for conversation in generator_b::generate(seed, variants)? {
        let mut history = Vec::new();
        let mut result = ConversationResult {
            id: conversation.id,
            class: conversation.class.into(),
            positive: conversation.positive,
            heavy_benign: conversation.heavy_benign,
            calls: conversation.calls,
            contact_events: 0,
            max_severity: 0,
            first_review_event: None,
            behavior_max: 0.0,
            text_only_max_severity: 0,
            keyword_flagged: false,
        };
        for event in conversation.events {
            let mut stored = event.stored;
            if stored.kind == hre_store::EventKind::Refusal {
                history.push(stored);
                continue;
            }
            let started = Instant::now();
            result.contact_events += 1;
            let mut text_input = FusionInput {
                simulated: true,
                ..FusionInput::default()
            };
            if let Some(text) = event.text {
                let report = hre_text::analyze(text)?;
                stored.toxicity = report.toxicity;
                stored.threat = report.threat;
                stored.sexual = report.sexual_explicit;
                stored.pii_mentions = report.pii_matches.len() as u32;
                stored.location_cues = report.location_cues as u32;
                text_input.signals.text = report.primary_signal;
                text_input.signals.pii = report.pii_signal;
                text_input.threat = report.threat;
                // Match the current API adapter: do not invent extra coercion/doxxing floors.
                text_input.degraded = report.degraded;
                result.keyword_flagged |= keyword_score(text) >= 0.5;
            }
            history.push(stored.clone());
            let behavior = hre_behavior::analyze(
                &history,
                &stored.sender,
                &stored.recipient,
                stored.timestamp_ms,
                &behavior_config,
            );
            let mut combined = text_input.clone();
            combined.signals.behavior = behavior.behavior;
            combined.signals.pile_on = behavior.pile_on;
            combined.after_refusal_count = behavior.after_refusal_count;
            combined.healthy_reciprocity = behavior.healthy_reciprocity;
            combined.reply_to_harasser = behavior.reply_to_harasser;
            combined.feed_protection = behavior.feed_protection;
            combined.degraded.extend(behavior.degraded);
            let fused = hre_fusion::fuse(&combined, &fusion_config);
            latencies.push(started.elapsed().as_secs_f64() * 1000.0);
            result.max_severity = result.max_severity.max(fused.severity);
            result.behavior_max = result.behavior_max.max(
                combined
                    .signals
                    .behavior
                    .score
                    .max(combined.signals.pile_on.score),
            );
            result.text_only_max_severity = result
                .text_only_max_severity
                .max(hre_fusion::fuse(&text_input, &fusion_config).severity);
            if fused.severity >= fusion_config.thresholds.review
                && result.first_review_event.is_none()
            {
                result.first_review_event = Some(result.contact_events);
            }
        }
        conversations.push(result);
    }
    let threshold = f64::from(fusion_config.thresholds.review) / 100.0;
    let text_rows: Vec<_> = text_results.iter().map(|r| (r.positive, r.score)).collect();
    let fusion_rows: Vec<_> = conversations
        .iter()
        .map(|r| (r.positive, f64::from(r.max_severity) / 100.0))
        .collect();
    let behavior_rows: Vec<_> = conversations
        .iter()
        .map(|r| (r.positive, r.behavior_max))
        .collect();
    let text_only: Vec<_> = conversations
        .iter()
        .map(|r| (r.positive, f64::from(r.text_only_max_severity) / 100.0))
        .collect();
    let mut modules = BTreeMap::from([
        (
            "text".into(),
            measured(&text_rows, 0.5, "authored_text_case"),
        ),
        (
            "behavior".into(),
            measured(&behavior_rows, 0.31, "conversation_max_behavior_or_pile_on"),
        ),
        (
            "fusion".into(),
            measured(&fusion_rows, threshold, "conversation_any_review"),
        ),
    ]);
    for (name, reason) in [
        ("ocr", "No pixel OCR backend and paired image/transcript evaluation corpus connected; metadata proxies do not measure OCR."),
        ("image", "No real image classifier and labeled image corpus connected; hash/override proxies cannot measure classifier accuracy."),
        ("audio", "No ASR/audio backend and paired waveform/transcript corpus connected."),
        ("call", "No standalone call classifier connected; call logs exercise actual hre-behavior rules only."),
    ] { modules.insert(name.into(), unavailable(reason)); }
    let ablations = BTreeMap::from([
        ("text_only".into(), measured(&text_only, threshold, "conversation_any_review")),
        ("text_behavior".into(), measured(&fusion_rows, threshold, "conversation_any_review")),
        ("plus_calls".into(), unavailable("Standalone call module unavailable; existing call events already contribute to behavior.")),
        ("full".into(), unavailable("OCR/image/audio/call classifiers unavailable in this evaluator; text_behavior is the full available engine path.")),
    ]);
    let mut slices = BTreeMap::new();
    for (name, selected) in [
        (
            "heavy_benign_messages",
            conversations
                .iter()
                .filter(|r| r.heavy_benign && !r.calls)
                .collect::<Vec<_>>(),
        ),
        (
            "heavy_benign_calls",
            conversations
                .iter()
                .filter(|r| r.heavy_benign && r.calls)
                .collect(),
        ),
        (
            "behavior_call_context",
            conversations.iter().filter(|r| r.calls).collect(),
        ),
        (
            "polite_stalker_system",
            conversations
                .iter()
                .filter(|r| r.class == "polite_stalker")
                .collect(),
        ),
    ] {
        let rows: Vec<_> = selected
            .iter()
            .map(|r| (r.positive, f64::from(r.max_severity) / 100.0))
            .collect();
        slices.insert(
            name.into(),
            measured(&rows, threshold, "conversation_any_review"),
        );
    }
    let polite_keyword: Vec<_> = conversations
        .iter()
        .filter(|r| r.class == "polite_stalker")
        .map(|r| (r.positive, f64::from(u8::from(r.keyword_flagged))))
        .collect();
    slices.insert(
        "polite_stalker_keyword".into(),
        measured(&polite_keyword, 0.5, "conversation_any_keyword"),
    );
    let baseline: Vec<_> = text_results
        .iter()
        .map(|r| (r.positive, r.keyword_score))
        .collect();
    slices.insert(
        "text_keyword_baseline".into(),
        measured(&baseline, 0.5, "authored_text_case"),
    );
    for language in ["en", "hi-Latn", "hi", "mr-like"] {
        let rows: Vec<_> = text_results
            .iter()
            .filter(|r| r.language == language)
            .map(|r| (r.positive, r.score))
            .collect();
        slices.insert(
            format!("text_{language}"),
            measured(&rows, 0.5, "authored_text_case"),
        );
    }
    let positive_conversations = conversations.iter().filter(|r| r.positive).count();
    let times: Vec<_> = conversations
        .iter()
        .filter(|r| r.positive)
        .filter_map(|r| r.first_review_event.map(|n| n as f64))
        .collect();
    let detected_conversations = times.len();
    Ok(Report {
        schema_version: 1, simulated: true, seed, variants, generator: "rust_generator_b_v1_phase_schedules".into(),
        limitations: [
            "Synthetic results only; no field accuracy, pretrained model inference or public benchmark claims.",
            "Generator B is independent Rust code and reads no training/ files or Generator A outputs; distribution separation is not statistically certified.",
            "Semantic labels are authored before scoring, not derived from engine scores. Ambiguous cases are deliberately benign stress cases.",
            "Engine-only in-memory replay; no HTTP, persistence, alerts, client C8, model registry or media overrides.",
            "Text uses explicit heuristic backend. Behavior includes rule/logistic fallback and pile-on; fusion uses actual public functions with serialized configs.",
            "Conversation confusion uses maximum score across both directions; labels describe whole synthetic conversations, not offender attribution.",
            "Time-to-detection is one-based contact events (including replies, excluding refusal markers) from conversation start; misses are null and excluded from percentiles.",
            "Latency is serial un-warmed debug/release-dependent wall time for text + history append + behavior + combined fusion; excludes generation, text-only ablation, IO and API.",
            "Small language slices are illustrative; mr-like authored text is not linguistically validated coverage.",
            "Cyberflasher excluded from scored generator classes: absent real media inference cannot supply a prediction. No C8/deepfake/synthetic-voice evaluation.",
        ].iter().map(|s| (*s).into()).collect(),
        behavior_config, fusion_config, modules, ablations, slices,
        detection: Detection { positive_conversations, detected_conversations,
            detection_rate: metrics::ratio(detected_conversations, positive_conversations),
            time_to_detection_events_p50: metrics::percentile(&times, 50), time_to_detection_events_p95: metrics::percentile(&times, 95),
            missed_conversations: positive_conversations - detected_conversations },
        latency: Latency { n: latencies.len(), p50_ms: metrics::percentile(&latencies, 50),
            p95_ms: metrics::percentile(&latencies, 95), p99_ms: metrics::percentile(&latencies, 99), scope: "serial_engine_replay_ms".into() },
        conversations, text_cases: text_results, ocr_cer: None, ocr_wer: None, asr_wer: None,
        cohens_kappa: None, slang_temporal_split: None,
        unavailable_reasons: BTreeMap::from([
            ("ocr_cer/ocr_wer".into(), "No pixel OCR predictions with independent reference transcripts.".into()),
            ("asr_wer".into(), "No ASR predictions or TTS waveform/reference pairs.".into()),
            ("cohens_kappa".into(), "No independent annotator label files supplied; use hre-eval kappa LABEL_A.json LABEL_B.json.".into()),
            ("slang_temporal_split".into(), "No slang feedback/adaptation engine connected; no before/after scores invented.".into()),
        ]),
    })
}

/// Persist aggregate reports and case IDs/scores; no raw identities or message bodies.
pub fn write_report(report: &Report, directory: &Path) -> Result<(), EvalError> {
    std::fs::create_dir_all(directory)?;
    let json = serde_json::to_vec_pretty(report)?;
    let mut markdown = String::from("# Engine evaluation\n\nSynthetic Generator B and fresh authored text; heuristic backends only. No C8.\n\n");
    markdown.push_str("| Evaluation | N | TP | FP | TN | FN | Precision | Recall | F1 | FPR |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (name, measurement) in report
        .modules
        .iter()
        .chain(&report.ablations)
        .chain(&report.slices)
    {
        if let Some(m) = &measurement.metrics {
            markdown.push_str(&format!(
                "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                m.n,
                m.confusion.tp,
                m.confusion.fp,
                m.confusion.tn,
                m.confusion.fn_,
                number(m.precision),
                number(m.recall),
                number(m.f1),
                number(m.fpr)
            ));
        } else {
            markdown.push_str(&format!(
                "| {name}: {} | null | null | null | null | null | null | null | null | null |\n",
                measurement.reason.as_deref().unwrap_or("unavailable")
            ));
        }
    }
    markdown.push_str(&format!("\nDetection: {}/{} ({}). TTD p50/p95: {}/{} contact events; {} missed.\n\nLatency p50/p95/p99: {}/{}/{} ms ({} samples).\n",
        report.detection.detected_conversations, report.detection.positive_conversations, number(report.detection.detection_rate),
        number(report.detection.time_to_detection_events_p50), number(report.detection.time_to_detection_events_p95), report.detection.missed_conversations,
        number(report.latency.p50_ms), number(report.latency.p95_ms), number(report.latency.p99_ms), report.latency.n));
    for (name, reason) in &report.unavailable_reasons {
        markdown.push_str(&format!("\n- {name}: null — {reason}\n"));
    }
    markdown.push_str("\nMethodology and limits:\n");
    for limitation in &report.limitations {
        markdown.push_str(&format!("\n- {limitation}\n"));
    }
    markdown.push_str("\nSee metrics.json for PR curves, thresholds, configs and case results. See docs/EVALUATION_SCHEMA.md for definitions.\n");
    std::fs::write(directory.join("metrics.json"), json)?;
    std::fs::write(directory.join("METRICS.md"), markdown)?;
    Ok(())
}

fn number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".into(), |v| format!("{v:.4}"))
}

#[cfg(test)]
mod tests;
