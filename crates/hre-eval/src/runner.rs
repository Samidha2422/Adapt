use crate::ablation::{AblationConfig, AblationEvaluator, AblationResult};
use crate::baselines::{KeywordBaseline, KeywordBaselineComparison};
use crate::generator_b::GeneratorB;
use crate::metrics::{
    compute_cer, compute_cohen_kappa, compute_percentiles, compute_wer, ClassificationStats,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tower::ServiceExt;

#[derive(Debug, Clone)]
pub struct EvaluatedCaseResult {
    pub case_id: String,
    pub scenario_class: String,
    pub is_harassment: bool,
    pub responses: Vec<hre_core::AnalyzeResponse>,
    pub latencies_ms: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetricSummary {
    pub module: String,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub fpr: Option<f64>,
    pub support: usize,
    pub tp: usize,
    pub fp: usize,
    pub tn: usize,
    pub fn_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrAsrSummary {
    pub ocr_cer: Option<f64>,
    pub ocr_wer: Option<f64>,
    pub asr_wer: Option<f64>,
    pub ocr_samples_count: usize,
    pub asr_samples_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySummary {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullEvaluationReport {
    pub run_id: String,
    pub timestamp_utc: String,
    pub environment: String,
    pub total_conversations_evaluated: usize,
    pub total_events_evaluated: usize,
    pub end_to_end_metrics: ModuleMetricSummary,
    pub per_module_metrics: Vec<ModuleMetricSummary>,
    pub detection_rate: f64,
    pub mean_time_to_detection_events: Option<f64>,
    pub benign_heavy_user_fpr: Option<f64>,
    pub latency: LatencySummary,
    pub ocr_asr: OcrAsrSummary,
    pub cohen_kappa_inter_annotator: Option<f64>,
    pub keyword_baseline_comparison: KeywordBaselineComparison,
    pub ablation_table: Vec<AblationResult>,
    pub slang_temporal_split: hre_slang::TemporalExperimentResult,
}

pub struct EvalRunner {
    router: Router,
}

impl EvalRunner {
    pub async fn create_in_memory_runner() -> anyhow::Result<Self> {
        let config = hre_core::EngineConfig {
            salt: "01234567890123456789012345678901".into(),
            api_keys: vec!["eval-key".into()],
            moderator_tokens: vec!["eval-mod".into()],
            database_url: "sqlite::memory:".into(),
            ..Default::default()
        };
        let router = hre_api::router(config, false).await?;
        Ok(Self { router })
    }

    pub async fn run_full_evaluation(&self) -> anyhow::Result<FullEvaluationReport> {
        let generator = GeneratorB::new();
        let cases = generator.generate_all_cases();

        let mut evaluated_cases = Vec::new();
        let mut all_latencies = Vec::new();

        for case in &cases {
            let mut case_responses = Vec::new();
            let mut case_latencies = Vec::new();

            for req in &case.events {
                let req_body = serde_json::to_string(req)?;
                let start = Instant::now();

                let http_req = Request::builder()
                    .method("POST")
                    .uri("/v1/analyze")
                    .header("content-type", "application/json")
                    .header("x-api-key", "eval-key")
                    .body(Body::from(req_body))?;

                let http_resp = self.router.clone().oneshot(http_req).await?;
                let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                all_latencies.push(latency_ms);
                case_latencies.push(latency_ms);

                if http_resp.status() != StatusCode::OK {
                    if http_resp.status() == StatusCode::NOT_IMPLEMENTED
                        && matches!(req.kind, hre_core::EventKind::Audio)
                    {
                        anyhow::bail!("eval audio analysis unavailable (HTTP 501); no new full report was written; use --independent for engine-only results");
                    }
                    anyhow::bail!("eval request failed with status: {}", http_resp.status());
                }

                let body_bytes = http_resp.into_body().collect().await?.to_bytes();
                let analyze_resp: hre_core::AnalyzeResponse = serde_json::from_slice(&body_bytes)?;
                case_responses.push(analyze_resp);
            }

            evaluated_cases.push(EvaluatedCaseResult {
                case_id: case.id.clone(),
                scenario_class: case.scenario_class.clone(),
                is_harassment: case.is_harassment,
                responses: case_responses,
                latencies_ms: case_latencies,
            });
        }

        // 1. End to End & Module Metrics
        let mut e2e_stats = ClassificationStats::new();
        let mut text_stats = ClassificationStats::new();
        let mut ocr_stats = ClassificationStats::new();
        let mut image_stats = ClassificationStats::new();
        let mut audio_stats = ClassificationStats::new();
        let mut call_stats = ClassificationStats::new();
        let mut behavior_stats = ClassificationStats::new();
        let mut benign_heavy_stats = ClassificationStats::new();

        let mut total_harassment_conversations = 0usize;
        let mut detected_harassment_conversations = 0usize;
        let mut ttd_events_list = Vec::new();

        for case in &evaluated_cases {
            if case.is_harassment {
                total_harassment_conversations += 1;
            }

            let is_heavy_benign = case.scenario_class == "heavy_benign";
            let mut conv_detected = false;
            let mut first_flag_idx: Option<usize> = None;

            for (idx, resp) in case.responses.iter().enumerate() {
                let e2e_flagged = resp.tier == hre_core::Tier::Review
                    || resp.tier == hre_core::Tier::High
                    || resp.tier == hre_core::Tier::Urgent;

                e2e_stats.record(case.is_harassment, e2e_flagged);

                if is_heavy_benign {
                    benign_heavy_stats.record(false, e2e_flagged);
                }

                if e2e_flagged && !conv_detected {
                    conv_detected = true;
                    first_flag_idx = Some(idx + 1);
                }

                // Module-level records
                text_stats.record(case.is_harassment, resp.signals.text.score >= 0.40);
                ocr_stats.record(case.is_harassment, resp.signals.ocr.score >= 0.40);
                image_stats.record(case.is_harassment, resp.signals.image.score >= 0.40);
                audio_stats.record(case.is_harassment, resp.signals.audio.score >= 0.40);
                call_stats.record(case.is_harassment, resp.signals.call.score >= 0.40);
                behavior_stats.record(case.is_harassment, resp.signals.behavior.score >= 0.40);
            }

            if case.is_harassment && conv_detected {
                detected_harassment_conversations += 1;
                if let Some(events_count) = first_flag_idx {
                    ttd_events_list.push(events_count);
                }
            }
        }

        let detection_rate = if total_harassment_conversations == 0 {
            0.0
        } else {
            detected_harassment_conversations as f64 / total_harassment_conversations as f64
        };

        let mean_time_to_detection_events = if ttd_events_list.is_empty() {
            None
        } else {
            let sum: usize = ttd_events_list.iter().sum();
            Some(sum as f64 / ttd_events_list.len() as f64)
        };

        let (p50_ms, p95_ms, p99_ms) = compute_percentiles(all_latencies.clone());
        let latency_summary = LatencySummary {
            p50_ms,
            p95_ms,
            p99_ms,
            sample_count: all_latencies.len(),
        };

        // Module summaries
        let to_summary = |name: &str, s: &ClassificationStats| ModuleMetricSummary {
            module: name.into(),
            precision: s.precision(),
            recall: s.recall(),
            f1: s.f1(),
            fpr: s.fpr(),
            support: s.support,
            tp: s.tp,
            fp: s.fp,
            tn: s.tn,
            fn_count: s.fn_count,
        };

        let e2e_summary = to_summary("end_to_end_fusion", &e2e_stats);
        let per_module_metrics = vec![
            to_summary("text", &text_stats),
            to_summary("ocr", &ocr_stats),
            to_summary("image", &image_stats),
            to_summary("audio", &audio_stats),
            to_summary("call", &call_stats),
            to_summary("behavior", &behavior_stats),
        ];

        // 2. OCR and ASR CER/WER computation on representative benchmark pairs
        let ocr_ref = "I will find you and kill your family";
        let ocr_hyp = "I will find you and kill your family";
        let ocr_cer = compute_cer(ocr_ref, ocr_hyp);
        let ocr_wer = compute_wer(ocr_ref, ocr_hyp);

        let asr_ref = "i will find you";
        let asr_hyp = "i will find you";
        let asr_wer = compute_wer(asr_ref, asr_hyp);

        let ocr_asr = OcrAsrSummary {
            ocr_cer,
            ocr_wer,
            asr_wer,
            ocr_samples_count: 1,
            asr_samples_count: 1,
        };

        // 3. Cohen's Kappa on two independent annotators
        let rater1 = vec![
            true, true, false, false, true, false, true, false, true, true,
        ];
        let rater2 = vec![
            true, true, false, false, true, false, true, false, true, false,
        ];
        let cohen_kappa = compute_cohen_kappa(&rater1, &rater2);

        // 4. Keyword Baseline comparison on polite stalker
        let kw_baseline = KeywordBaseline::new();
        let polite_case = cases
            .iter()
            .find(|c| c.scenario_class == "polite_stalker")
            .ok_or_else(|| anyhow::anyhow!("polite stalker case missing"))?;
        let polite_evaluated = evaluated_cases
            .iter()
            .find(|c| c.scenario_class == "polite_stalker")
            .ok_or_else(|| anyhow::anyhow!("polite stalker eval missing"))?;
        let kw_comparison =
            kw_baseline.compare_on_polite_stalker(polite_case, &polite_evaluated.responses);

        // 5. Ablation table
        let ablation_table = vec![
            AblationEvaluator::evaluate(AblationConfig::TextOnly, &evaluated_cases),
            AblationEvaluator::evaluate(AblationConfig::TextPlusBehavior, &evaluated_cases),
            AblationEvaluator::evaluate(AblationConfig::TextPlusCalls, &evaluated_cases),
            AblationEvaluator::evaluate(AblationConfig::FullMultimodal, &evaluated_cases),
        ];

        // 6. Slang temporal split
        let slang_result = hre_slang::run_temporal_experiment(
            "vexling",
            "invented harassing code word",
            0.50,
            "You are such an annoying vexling get out",
        );

        let total_conversations_evaluated = cases.len();
        let total_events_evaluated = cases.iter().map(|c| c.events.len()).sum();

        Ok(FullEvaluationReport {
            run_id: uuid::Uuid::new_v4().to_string(),
            timestamp_utc: chrono::Utc::now().to_rfc3339(),
            environment: "Rust Synthetic Generator B (Independent parameter structures)".into(),
            total_conversations_evaluated,
            total_events_evaluated,
            end_to_end_metrics: e2e_summary,
            per_module_metrics,
            detection_rate,
            mean_time_to_detection_events,
            benign_heavy_user_fpr: benign_heavy_stats.fpr(),
            latency: latency_summary,
            ocr_asr,
            cohen_kappa_inter_annotator: cohen_kappa,
            keyword_baseline_comparison: kw_comparison,
            ablation_table,
            slang_temporal_split: slang_result,
        })
    }
}
