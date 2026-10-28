//! Multi-label text scoring and trait-based classification.

use crate::{
    cues::{detect_cues, CueMatches},
    normalizer::{normalize, normalize_pii_input, MAX_TEXT_LENGTH_BYTES},
    pii::{mask_pii, PiiMatch},
    TextError,
};
use hre_core::{Backend, Reason, Signal, Status};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextReport {
    pub primary_signal: Signal,
    pub pii_signal: Signal,
    pub masked_text: String,
    pub pii_matches: Vec<PiiMatch>,
    pub toxicity: f64,
    pub severe_toxicity: f64,
    pub obscene: f64,
    pub identity_attack: f64,
    pub insult: f64,
    pub threat: f64,
    pub sexual_explicit: f64,
    pub coercion: f64,
    pub location_cues: usize,
    pub reasons: Vec<Reason>,
    pub degraded: Vec<String>,
}

/// Pluggable text inference backend trait.
pub trait TextBackend: Send + Sync {
    fn analyze(&self, text: &str) -> Result<TextReport, TextError>;
}

/// Deterministic model-free heuristic backend.
#[derive(Debug, Clone, Default)]
pub struct HeuristicTextBackend;

impl TextBackend for HeuristicTextBackend {
    fn analyze(&self, raw_text: &str) -> Result<TextReport, TextError> {
        let byte_len = raw_text.len();
        if byte_len > MAX_TEXT_LENGTH_BYTES {
            return Err(TextError::TextTooLong {
                len: byte_len,
                max: MAX_TEXT_LENGTH_BYTES,
            });
        }
        if raw_text.trim().is_empty() {
            return Err(TextError::EmptyText);
        }

        // 1. Normalize
        let normalized = normalize(raw_text);
        let lower = normalized.to_ascii_lowercase();

        // 2. PII Detection & Masking
        // Preserve identifier repetitions while retaining Unicode canonicalization.
        let pii_input = normalize_pii_input(raw_text);
        let (masked_text, pii_matches) = mask_pii(&pii_input);

        // 3. Cue Detection
        let cues = detect_cues(&lower);

        // 4. Calculate Scores
        let mut report = compute_heuristic_scores(&lower, &cues, &pii_matches);
        report.masked_text = masked_text;
        report.pii_matches = pii_matches;
        report.location_cues = cues.location_cues.len();

        Ok(report)
    }
}

fn compute_heuristic_scores(
    lower: &str,
    cues: &CueMatches,
    pii_matches: &[PiiMatch],
) -> TextReport {
    let mut reasons = Vec::new();

    let mut threat: f64 = if !cues.threat_cues.is_empty() {
        0.95
    } else {
        0.0
    };

    let mut coercion: f64 = if !cues.coercion_cues.is_empty() {
        0.90
    } else {
        0.0
    };

    let mut insult: f64 = 0.0;
    const INSULT_WORDS: &[&str] = &[
        "idiot",
        "stupid",
        "moron",
        "loser",
        "dumb",
        "ugly",
        "scum",
        "trash",
        "worthless",
        "bastard",
        "bitch",
    ];
    for &w in INSULT_WORDS {
        if lower.contains(w) {
            insult = insult.max(0.70);
        }
    }

    let mut obscene: f64 = 0.0;
    const OBSCENE_WORDS: &[&str] = &[
        "fuck", "shit", "dick", "asshole", "pussy", "cunt", "slut", "whore",
    ];
    for &w in OBSCENE_WORDS {
        if lower.contains(w) {
            obscene = obscene.max(0.85);
        }
    }

    let mut sexual_explicit: f64 = 0.0;
    const SEXUAL_WORDS: &[&str] = &[
        "send nudes",
        "show boobs",
        "strip for me",
        "naked photos",
        "sexual acts",
        "suck my",
    ];
    for &w in SEXUAL_WORDS {
        if lower.contains(w) {
            sexual_explicit = sexual_explicit.max(0.88);
        }
    }

    let mut identity_attack: f64 = 0.0;
    const HATE_WORDS: &[&str] = &[
        "chamar",
        "bhangi",
        "katwe",
        "jihadi",
        "go back to your country",
        "filthy immigrant",
    ];
    for &w in HATE_WORDS {
        if lower.contains(w) {
            identity_attack = identity_attack.max(0.92);
        }
    }

    let mut toxicity: f64 = threat
        .max(coercion)
        .max(insult)
        .max(obscene)
        .max(sexual_explicit)
        .max(identity_attack);

    // Apply Quoted-Abuse discount if the text is clearly quoted / reported speech
    if cues.is_quoted && toxicity > 0.3 {
        toxicity *= 0.35;
        threat *= 0.35;
        coercion *= 0.35;
        insult *= 0.35;
        reasons.push(Reason {
            code: "QUOTED_ABUSE_DISCOUNT".into(),
            text: "Content is quoted speech or reported context, discounted hostility score".into(),
            weight: 0.2,
            evidence: BTreeMap::new(),
        });
    }

    let severe_toxicity = if threat > 0.8 || (toxicity > 0.8 && obscene > 0.7) {
        0.88
    } else {
        0.0
    };

    // Reasons generation
    if threat >= 0.8 {
        reasons.push(Reason {
            code: "DIRECT_THREAT_DETECTED".into(),
            text: format!("Direct threat pattern detected: {:?}", cues.threat_cues),
            weight: 0.95,
            evidence: BTreeMap::new(),
        });
    }
    if coercion >= 0.8 {
        reasons.push(Reason {
            code: "COERCION_SEXTORTION_CUE".into(),
            text: format!("Coercion or extortion pattern: {:?}", cues.coercion_cues),
            weight: 0.90,
            evidence: BTreeMap::new(),
        });
    }
    if !cues.location_cues.is_empty() {
        reasons.push(Reason {
            code: "LOCATION_STALKING_CUE".into(),
            text: format!("Physical presence / location cue: {:?}", cues.location_cues),
            weight: 0.75,
            evidence: BTreeMap::new(),
        });
    }

    // PII signal and Doxxing detection
    let has_pii = !pii_matches.is_empty();
    let is_doxxing = has_pii && (toxicity >= 0.6 || threat >= 0.6 || coercion >= 0.6);
    let pii_score = if is_doxxing {
        reasons.push(Reason {
            code: "DOXXING_WITH_HOSTILITY".into(),
            text: "Personal identifiable information posted in a hostile or threatening context"
                .into(),
            weight: 0.85,
            evidence: BTreeMap::new(),
        });
        0.90
    } else if has_pii {
        reasons.push(Reason {
            code: "PII_DETECTED".into(),
            text: format!("Detected {} personal information items", pii_matches.len()),
            weight: 0.45,
            evidence: BTreeMap::new(),
        });
        0.60
    } else {
        0.0
    };

    let mut details = BTreeMap::new();
    details.insert("toxicity".into(), serde_json::json!(toxicity));
    details.insert("threat".into(), serde_json::json!(threat));
    details.insert("obscene".into(), serde_json::json!(obscene));
    details.insert("insult".into(), serde_json::json!(insult));
    details.insert("coercion".into(), serde_json::json!(coercion));

    let primary_signal = Signal {
        score: toxicity,
        label: "text_heuristic".into(),
        status: Status::Built,
        backend: Backend::Heuristic,
        confidence: if toxicity > 0.0 { 0.75 } else { 0.50 },
        details,
    };

    let pii_signal = Signal {
        score: pii_score,
        label: "pii_heuristic".into(),
        status: Status::Built,
        backend: Backend::Heuristic,
        confidence: if has_pii { 0.90 } else { 0.50 },
        details: BTreeMap::from([
            ("match_count".into(), serde_json::json!(pii_matches.len())),
            ("is_doxxing".into(), serde_json::json!(is_doxxing)),
        ]),
    };

    TextReport {
        primary_signal,
        pii_signal,
        masked_text: String::new(),
        pii_matches: Vec::new(),
        toxicity,
        severe_toxicity,
        obscene,
        identity_attack,
        insult,
        threat,
        sexual_explicit,
        coercion,
        location_cues: cues.location_cues.len(),
        reasons,
        degraded: vec!["text.onnx_missing".into(), "text.heuristic_fallback".into()],
    }
}
