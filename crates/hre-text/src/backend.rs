//! Deterministic text classification backend and multi-label toxicity scoring.
//!
//! Exposes the `TextBackend` trait, the `HeuristicTextBackend` implementation,
//! and typed multi-label toxicity structures matching Detoxify categories:
//! - toxicity
//! - severe_toxicity
//! - obscene
//! - identity_attack
//! - insult
//! - threat
//! - sexual_explicit

use crate::{
    cues::{analyze_cues, CueMatches},
    error::TextError,
    normalize::{fold_leetspeak, normalize, validate_text},
    pii::{detect_and_mask_pii, PiiDetectionResult},
};
use hre_core::{Backend, Signal, Status};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Multi-label toxicity scores matching Detoxify label definitions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MultiLabelScores {
    pub toxicity: f64,
    pub severe_toxicity: f64,
    pub obscene: f64,
    pub identity_attack: f64,
    pub insult: f64,
    pub threat: f64,
    pub sexual_explicit: f64,
}

/// Comprehensive analysis result produced by the text pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnalysisResult {
    /// Canonical `Signal` suitable for consumption by `hre-api` and `hre-fusion`.
    pub signal: Signal,
    /// Detailed multi-label score breakdown.
    pub labels: MultiLabelScores,
    /// Inferred language code (e.g. "en", "hi", "hi-Latn").
    pub language: String,
    /// PII detection and masking results.
    pub pii: PiiDetectionResult,
    /// Extracted behavioral, coercion, and threat cues.
    pub cues: CueMatches,
    /// Module degradation notices (e.g. `["text.onnx_missing"]`).
    pub degraded: Vec<String>,
}

/// Pluggable backend trait for text inference (ONNX or deterministic heuristic).
pub trait TextBackend: Send + Sync {
    /// Returns the core `Signal` for standard API integration.
    fn analyze(&self, text: &str) -> Result<Signal, TextError>;

    /// Returns detailed multi-label, PII, cue, and degradation information.
    fn analyze_detailed(&self, text: &str) -> Result<TextAnalysisResult, TextError>;
}

/// Common insult lexicon (English and Hinglish).
const INSULT_TERMS: &[&str] = &[
    "bitch",
    "idiot",
    "loser",
    "ugly",
    "stupid",
    "moron",
    "pathetic",
    "worthless",
    "dumb",
    "disgusting",
    "freak",
    "clown",
    "trash",
    "scum",
    "whore",
    "slut",
    // Hinglish insults
    "saala",
    "saale",
    "kutta",
    "kutti",
    "kamina",
    "kamine",
    "gadha",
    "chirkut",
];

/// Common profanity / obscene terms.
const OBSCENE_TERMS: &[&str] = &[
    "fuck", "fucking", "fucked", "fucker", "shit", "bullshit", "asshole", "bastard", "cunt",
    "dick", "pussy",
];

/// Identity attack keywords.
const IDENTITY_TERMS: &[&str] = &[
    "retard", "faggot", "dyke", "tranny", "nigger", "chink", "paki",
];

/// Sexual / explicit content terms.
const SEXUAL_EXPLICIT_TERMS: &[&str] = &[
    "send nudes",
    "send nude",
    "show boobs",
    "send pussy",
    "send dick",
    "blowjob",
    "rape you",
    "naked photos",
    "intimate pictures",
];

/// Hinglish lexical markers for heuristic language identification.
const HINGLISH_MARKERS: &[&str] = &[
    "kya", "nahi", "tere", "mera", "meri", "hum", "tum", "bhai", "yaar", "saala", "kamine", "mar",
    "chup", "dekh", "aaja", "karo", "raha", "rahi", "hai",
];

/// Heuristic model-free text analysis backend.
///
/// Guaranteed to run deterministically with zero model files or network access,
/// reporting `Status::Built`, `Backend::Heuristic`, and degradation notice `text.onnx_missing`.
#[derive(Debug, Clone, Default)]
pub struct HeuristicTextBackend;

impl HeuristicTextBackend {
    pub fn new() -> Self {
        Self
    }

    /// Infers language based on Unicode script block and regional lexical markers.
    pub fn detect_language(text: &str) -> String {
        // Check for Devanagari script (Hindi, Marathi, Konkani)
        if text.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c)) {
            return "hi".to_string();
        }

        // Check for Hinglish markers in Latin text
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        let hinglish_count = words
            .iter()
            .filter(|w| HINGLISH_MARKERS.contains(&w.trim_matches(|c: char| !c.is_alphabetic())))
            .count();

        if hinglish_count >= 2 {
            "hi-Latn".to_string()
        } else {
            "en".to_string()
        }
    }
}

impl TextBackend for HeuristicTextBackend {
    fn analyze(&self, text: &str) -> Result<Signal, TextError> {
        let detailed = self.analyze_detailed(text)?;
        Ok(detailed.signal)
    }

    fn analyze_detailed(&self, text: &str) -> Result<TextAnalysisResult, TextError> {
        // Enforce input bounds
        validate_text(text)?;

        // 1. Conservative normalization
        let norm = normalize(text);
        let leet = fold_leetspeak(&norm);
        let lower = leet.to_lowercase();

        // 2. Language identification
        let language = Self::detect_language(&norm);

        // 3. PII Detection and masking
        let pii = detect_and_mask_pii(&norm);

        // 4. Cue analysis (threat, coercion, location, sextortion, quoted abuse)
        let cues = analyze_cues(&norm);

        // 5. Multi-label scoring
        let mut labels = MultiLabelScores {
            threat: cues.threat_score,
            ..MultiLabelScores::default()
        };

        // Insult score
        let insult_matches = INSULT_TERMS
            .iter()
            .filter(|&&term| lower.contains(term))
            .count();
        if insult_matches > 0 {
            labels.insult = (0.4 + (insult_matches as f64) * 0.2).min(0.9);
        }

        // Obscene score
        let obscene_matches = OBSCENE_TERMS
            .iter()
            .filter(|&&term| lower.contains(term))
            .count();
        if obscene_matches > 0 {
            labels.obscene = (0.4 + (obscene_matches as f64) * 0.2).min(0.9);
        }

        // Identity attack score
        let identity_matches = IDENTITY_TERMS
            .iter()
            .filter(|&&term| lower.contains(term))
            .count();
        if identity_matches > 0 {
            labels.identity_attack = 0.95;
        }

        // Sexual explicit score
        let sexual_matches = SEXUAL_EXPLICIT_TERMS
            .iter()
            .filter(|&&term| lower.contains(term))
            .count();
        if sexual_matches > 0 {
            labels.sexual_explicit = 0.9;
        }

        // Overall toxicity: noisy-OR / max aggregation
        let mut max_category = labels
            .threat
            .max(labels.identity_attack)
            .max(labels.sexual_explicit)
            .max(labels.insult)
            .max(labels.obscene);

        // Severe toxicity
        if labels.threat >= 0.85
            || (max_category >= 0.85 && (labels.insult >= 0.7 || labels.obscene >= 0.7))
        {
            labels.severe_toxicity = 0.9;
        }

        // Quoted abuse discount (C5 requirement: quoted abuse is discounted)
        if cues.quoted_abuse {
            max_category = (max_category * 0.3).min(0.35);
            labels.insult *= 0.3;
            labels.obscene *= 0.3;
            labels.severe_toxicity *= 0.3;
        }

        labels.toxicity = max_category;

        // Dominant label
        let dominant_label = if labels.threat >= 0.8 {
            "threat"
        } else if labels.identity_attack >= 0.8 {
            "identity_attack"
        } else if labels.sexual_explicit >= 0.8 {
            "sexual_explicit"
        } else if labels.insult >= 0.5 {
            "insult"
        } else if labels.obscene >= 0.5 {
            "obscene"
        } else if labels.toxicity >= 0.4 {
            "toxic"
        } else {
            "benign"
        };

        // Confidence: heuristic baseline is calibrated conservatively
        let confidence = if labels.threat >= 0.9 {
            0.85
        } else if dominant_label != "benign" {
            0.55
        } else {
            0.4
        };

        let degraded = vec!["text.onnx_missing".to_string()];

        // Build details map for Signal
        let mut details = BTreeMap::new();
        details.insert("toxicity".to_string(), serde_json::json!(labels.toxicity));
        details.insert("threat".to_string(), serde_json::json!(labels.threat));
        details.insert("insult".to_string(), serde_json::json!(labels.insult));
        details.insert("obscene".to_string(), serde_json::json!(labels.obscene));
        details.insert(
            "identity_attack".to_string(),
            serde_json::json!(labels.identity_attack),
        );
        details.insert(
            "sexual_explicit".to_string(),
            serde_json::json!(labels.sexual_explicit),
        );
        details.insert("language".to_string(), serde_json::json!(language));
        details.insert(
            "masked_text".to_string(),
            serde_json::json!(pii.masked_text),
        );
        details.insert("has_pii".to_string(), serde_json::json!(pii.has_pii));
        details.insert(
            "quoted_abuse".to_string(),
            serde_json::json!(cues.quoted_abuse),
        );
        details.insert(
            "sextortion_pattern".to_string(),
            serde_json::json!(cues.sextortion_pattern),
        );

        let signal = Signal {
            score: labels.toxicity,
            label: dominant_label.to_string(),
            status: Status::Built,
            backend: Backend::Heuristic,
            confidence,
            details,
        };

        Ok(TextAnalysisResult {
            signal,
            labels,
            language,
            pii,
            cues,
            degraded,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_model_degraded() -> Result<(), TextError> {
        let backend = HeuristicTextBackend::new();
        let res = backend.analyze_detailed("Hello world")?;

        assert_eq!(res.signal.status, Status::Built);
        assert_eq!(res.signal.backend, Backend::Heuristic);
        assert!(
            res.degraded.contains(&"text.onnx_missing".to_string()),
            "Heuristic fallback must report text.onnx_missing in degraded list"
        );
        assert_eq!(res.signal.label, "benign");
        assert!(res.signal.score < 0.1);
        Ok(())
    }

    #[test]
    fn test_explicit_threat_scoring() -> Result<(), TextError> {
        let backend = HeuristicTextBackend::new();
        let res = backend.analyze_detailed("I will kill you")?;

        assert_eq!(res.signal.label, "threat");
        assert!(res.labels.threat >= 0.9);
        assert!(res.signal.score >= 0.9);
        Ok(())
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(
            HeuristicTextBackend::detect_language("Hello how are you?"),
            "en"
        );
        assert_eq!(HeuristicTextBackend::detect_language("नमस्ते आप कैसे हैं?"), "hi");
        assert_eq!(
            HeuristicTextBackend::detect_language("kya haal hai bhai, sab theek?"),
            "hi-Latn"
        );
    }
}
