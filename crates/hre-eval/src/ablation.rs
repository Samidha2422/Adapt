use crate::metrics::ClassificationStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationResult {
    pub configuration: String,
    pub description: String,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub fpr: Option<f64>,
    pub detection_rate: f64,
    pub mean_ttd_events: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AblationConfig {
    TextOnly,
    TextPlusBehavior,
    TextPlusCalls,
    FullMultimodal,
}

impl AblationConfig {
    pub fn name(&self) -> &'static str {
        match self {
            Self::TextOnly => "text_only",
            Self::TextPlusBehavior => "text_plus_behavior",
            Self::TextPlusCalls => "text_plus_calls",
            Self::FullMultimodal => "full_multimodal",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::TextOnly => "Text model and PII detector only; zero behavior state or call logs",
            Self::TextPlusBehavior => "Text model + directional pair history + pile-on detector",
            Self::TextPlusCalls => "Text model + call log behavior (volume, overnight, unanswered)",
            Self::FullMultimodal => {
                "Full pipeline: text, vision/OCR, audio, calls, behavior, and fusion"
            }
        }
    }

    /// Determines if an event would trigger an alert (>= Review tier) under this ablation configuration.
    pub fn evaluate_response(&self, resp: &hre_core::AnalyzeResponse) -> bool {
        match self {
            Self::TextOnly => {
                // Only consider text and pii scores
                let max_score = resp.signals.text.score.max(resp.signals.pii.score);
                max_score >= 0.40
            }
            Self::TextPlusBehavior => {
                let max_score = resp
                    .signals
                    .text
                    .score
                    .max(resp.signals.pii.score)
                    .max(resp.signals.behavior.score)
                    .max(resp.signals.pile_on.score);
                max_score >= 0.40
            }
            Self::TextPlusCalls => {
                let max_score = resp
                    .signals
                    .text
                    .score
                    .max(resp.signals.pii.score)
                    .max(resp.signals.call.score);
                max_score >= 0.40
            }
            Self::FullMultimodal => {
                // Tier review, high, or urgent
                resp.tier == hre_core::Tier::Review
                    || resp.tier == hre_core::Tier::High
                    || resp.tier == hre_core::Tier::Urgent
            }
        }
    }
}

pub struct AblationEvaluator;

impl AblationEvaluator {
    pub fn evaluate(
        config: AblationConfig,
        case_results: &[crate::runner::EvaluatedCaseResult],
    ) -> AblationResult {
        let mut stats = ClassificationStats::new();
        let mut total_harassment_conversations = 0usize;
        let mut detected_harassment_conversations = 0usize;
        let mut ttd_events_list = Vec::new();

        for case in case_results {
            if case.is_harassment {
                total_harassment_conversations += 1;
            }

            let mut conversation_flagged = false;
            let mut first_flag_event_idx: Option<usize> = None;

            for (idx, resp) in case.responses.iter().enumerate() {
                let is_positive = config.evaluate_response(resp);
                stats.record(case.is_harassment, is_positive);

                if is_positive && !conversation_flagged {
                    conversation_flagged = true;
                    first_flag_event_idx = Some(idx + 1);
                }
            }

            if case.is_harassment && conversation_flagged {
                detected_harassment_conversations += 1;
                if let Some(events_count) = first_flag_event_idx {
                    ttd_events_list.push(events_count);
                }
            }
        }

        let detection_rate = if total_harassment_conversations == 0 {
            0.0
        } else {
            detected_harassment_conversations as f64 / total_harassment_conversations as f64
        };

        let mean_ttd_events = if ttd_events_list.is_empty() {
            None
        } else {
            let sum: usize = ttd_events_list.iter().sum();
            Some(sum as f64 / ttd_events_list.len() as f64)
        };

        AblationResult {
            configuration: config.name().to_string(),
            description: config.description().to_string(),
            precision: stats.precision(),
            recall: stats.recall(),
            f1: stats.f1(),
            fpr: stats.fpr(),
            detection_rate,
            mean_ttd_events,
        }
    }
}
