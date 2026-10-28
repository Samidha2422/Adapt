//! Weighted noisy-OR with explicit safety floors, contextual caps and honest missing modules.
use hre_core::{Action, Backend, Reason, SecondaryAction, Signals, Status, Tier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Thresholds {
    pub review: u8,
    pub high: u8,
    pub urgent: u8,
}
impl Default for Thresholds {
    fn default() -> Self {
        Self {
            review: 31,
            high: 61,
            urgent: 86,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Floors {
    pub threat: u8,
    pub contact_after_refusal: u8,
    pub repeated_after_refusal: u8,
    pub unsolicited_explicit: u8,
    pub reported_hash_match: u8,
    pub sextortion: u8,
    pub synthetic_intimate: u8,
    pub doxxing: u8,
    pub address_threat: u8,
    pub pile_on: u8,
}
impl Default for Floors {
    fn default() -> Self {
        Self {
            threat: 86,
            contact_after_refusal: 61,
            repeated_after_refusal: 75,
            unsolicited_explicit: 61,
            reported_hash_match: 86,
            sextortion: 86,
            synthetic_intimate: 86,
            doxxing: 70,
            address_threat: 86,
            pile_on: 61,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Caps {
    pub healthy_reciprocity: u8,
    pub quoted_abuse_discount: f64,
    pub banter_discount: f64,
    pub tone_max_addition: f64,
}
impl Default for Caps {
    fn default() -> Self {
        Self {
            healthy_reciprocity: 55,
            quoted_abuse_discount: 0.5,
            banter_discount: 0.7,
            tone_max_addition: 10.0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FusionConfig {
    pub thresholds: Thresholds,
    pub weights: BTreeMap<String, f64>,
    pub floors: Floors,
    pub caps: Caps,
    /// Optional monotone [input, output] severity knots; empty means uncalibrated.
    pub calibration: Vec<[f64; 2]>,
}
impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            thresholds: Thresholds::default(),
            floors: Floors::default(),
            caps: Caps::default(),
            calibration: Vec::new(),
            weights: BTreeMap::from([
                ("text".into(), 0.8),
                ("ocr".into(), 0.8),
                ("image".into(), 0.85),
                ("audio".into(), 0.85),
                ("call".into(), 0.8),
                ("behavior".into(), 0.85),
                ("pile_on".into(), 0.85),
                ("pii".into(), 0.4),
            ]),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum FusionError {
    #[error("fusion TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid fusion configuration")]
    InvalidConfig,
}
impl FusionConfig {
    pub fn parse(toml: &str) -> Result<Self, FusionError> {
        let c: Self = toml::from_str(toml)?;
        c.validate()?;
        Ok(c)
    }
    pub fn validate(&self) -> Result<(), FusionError> {
        let t = &self.thresholds;
        let c = &self.caps;
        let f = &self.floors;
        if !(0 < t.review && t.review < t.high && t.high < t.urgent && t.urgent <= 100)
            || self.weights.iter().any(|(k, v)| {
                ![
                    "text", "ocr", "image", "audio", "call", "behavior", "pile_on", "pii",
                ]
                .contains(&k.as_str())
                    || !v.is_finite()
                    || !(0.0..=1.0).contains(v)
            })
            || [
                f.threat,
                f.contact_after_refusal,
                f.repeated_after_refusal,
                f.unsolicited_explicit,
                f.reported_hash_match,
                f.sextortion,
                f.synthetic_intimate,
                f.doxxing,
                f.address_threat,
                f.pile_on,
                c.healthy_reciprocity,
            ]
            .iter()
            .any(|v| *v > 100)
            || [c.quoted_abuse_discount, c.banter_discount]
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || !c.tone_max_addition.is_finite()
            || !(0.0..=10.0).contains(&c.tone_max_addition)
            || (!self.calibration.is_empty()
                && (self.calibration.len() < 2
                    || self.calibration.first().map(|p| p[0]) != Some(0.0)
                    || self.calibration.last().map(|p| p[0]) != Some(100.0)))
            || self
                .calibration
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=100.0).contains(v))
            || self
                .calibration
                .windows(2)
                .any(|p| p[0][0] >= p[1][0] || p[0][1] > p[1][1])
        {
            return Err(FusionError::InvalidConfig);
        }
        Ok(())
    }
}
/// Safety/context facts must come from module detections or authenticated ingest context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FusionInput {
    pub signals: Signals,
    pub reasons: Vec<Reason>,
    pub degraded: Vec<String>,
    pub threat: f64,
    pub sextortion_pattern: bool,
    pub reported_hash_match: bool,
    pub synthetic_intimate: bool,
    pub unsolicited_explicit: bool,
    pub after_refusal_count: usize,
    pub doxxing_with_toxicity: bool,
    pub address_with_threat: bool,
    pub healthy_reciprocity: bool,
    pub reply_to_harasser: bool,
    pub quoted_abuse: bool,
    pub banter: bool,
    pub tone: f64,
    pub feed_protection: bool,
    pub simulated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionReport {
    pub severity: u8,
    pub tier: Tier,
    pub reasons: Vec<Reason>,
    pub recommended_action: Action,
    pub secondary_actions: Vec<SecondaryAction>,
    pub simulated: bool,
    pub degraded: Vec<String>,
}
fn unit(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
fn reason(code: &str, text: &str, before: f64, after: f64) -> Reason {
    Reason {
        code: code.into(),
        text: text.into(),
        weight: (after - before) / 100.0,
        evidence: BTreeMap::from([
            ("before".into(), serde_json::json!(before)),
            ("after".into(), serde_json::json!(after)),
        ]),
    }
}
fn calibrate(score: f64, knots: &[[f64; 2]]) -> f64 {
    for p in knots.windows(2) {
        if score <= p[1][0] {
            return p[0][1] + (score - p[0][0]) / (p[1][0] - p[0][0]) * (p[1][1] - p[0][1]);
        }
    }
    score
}
/// Caps apply to soft evidence; explicit safety floors apply afterwards and cannot be discounted.
pub fn fuse(input: &FusionInput, config: &FusionConfig) -> FusionReport {
    let defaults = FusionConfig::default();
    let c = if config.validate().is_ok() {
        config
    } else {
        &defaults
    };
    let mut degraded = input.degraded.clone();
    if config.validate().is_err() {
        degraded.push("fusion.invalid_config_defaults_used".into());
    }
    let mut reasons = input.reasons.clone();
    let s = &input.signals;
    let modules = [
        ("text", &s.text),
        ("ocr", &s.ocr),
        ("image", &s.image),
        ("audio", &s.audio),
        ("call", &s.call),
        ("behavior", &s.behavior),
        ("pile_on", &s.pile_on),
        ("pii", &s.pii),
    ];
    let mut product = 1.0;
    let mut simulated = input.simulated;
    for (name, signal) in modules {
        if signal.backend == Backend::Disabled {
            degraded.push(format!("{name}.unavailable"));
            continue;
        }
        if !signal.score.is_finite() || !(0.0..=1.0).contains(&signal.score) {
            degraded.push(format!("{name}.invalid_score"));
        }
        if input.reply_to_harasser && ["text", "ocr", "audio", "behavior"].contains(&name) {
            continue;
        }
        let contribution = c.weights.get(name).copied().unwrap_or(0.0) * unit(signal.score);
        product *= 1.0 - contribution;
        if contribution > 0.0 && signal.status == Status::Simulated {
            simulated = true;
        }
    }
    let mut score = 100.0 * (1.0 - product);
    if !c.calibration.is_empty() {
        let before = score;
        score = calibrate(score, &c.calibration);
        reasons.push(reason(
            "CALIBRATION",
            "Configured monotone calibration applied",
            before,
            score,
        ));
    }
    if input.reply_to_harasser {
        reasons.push(reason(
            "REPLY_TO_HARASSER",
            "Hostile reply is contextualized by the reverse harassment history",
            score,
            score,
        ));
    }
    if input.quoted_abuse {
        let before = score;
        score *= c.caps.quoted_abuse_discount;
        reasons.push(reason(
            "QUOTED_ABUSE_DISCOUNT",
            "Quoted abuse receives reduced weight",
            before,
            score,
        ));
    }
    if input.banter {
        let before = score;
        score *= c.caps.banter_discount;
        reasons.push(reason(
            "BANTER_DISCOUNT",
            "Reciprocal banter receives reduced weight",
            before,
            score,
        ));
    }
    if score > 0.0 && unit(input.tone) > 0.0 {
        let before = score;
        score = (score + unit(input.tone) * c.caps.tone_max_addition).min(100.0);
        if before < f64::from(c.thresholds.review) {
            score = score.min(f64::from(c.thresholds.review - 1));
        }
        reasons.push(reason(
            "SUPPORTING_TONE",
            "Vocal tone adds limited supporting evidence",
            before,
            score,
        ));
    }
    if input.healthy_reciprocity && unit(input.threat) <= 0.8 {
        let before = score;
        score = score.min(f64::from(c.caps.healthy_reciprocity));
        if before > score {
            reasons.push(reason(
                "HEALTHY_RECIPROCITY_CAP",
                "Reciprocal contact caps contextual risk",
                before,
                score,
            ));
        }
    }
    let f = &c.floors;
    for (fired, floor, code, text) in [
        (
            !input.reply_to_harasser && unit(input.threat) >= 0.9,
            f.threat,
            "THREAT_FLOOR",
            "Explicit threat requires immediate human review",
        ),
        (
            input.sextortion_pattern,
            f.sextortion,
            "SEXTORTION_PATTERN",
            "Coercion and payment or leak cues require review",
        ),
        (
            input.reported_hash_match,
            f.reported_hash_match,
            "REPORTED_HASH_MATCH",
            "Media matches a reported test hash",
        ),
        (
            input.synthetic_intimate,
            f.synthetic_intimate,
            "POSSIBLE_SYNTHETIC_INTIMATE_MEDIA",
            "Possible synthetic intimate media; low confidence, human review required",
        ),
        (
            input.unsolicited_explicit,
            f.unsolicited_explicit,
            "UNSOLICITED_EXPLICIT_MEDIA",
            "Unsolicited explicit media should be blurred",
        ),
        (
            input.after_refusal_count > 0,
            f.contact_after_refusal,
            "CONTACT_AFTER_REFUSAL",
            "Contact continued after refusal",
        ),
        (
            input.after_refusal_count >= 3,
            f.repeated_after_refusal,
            "REPEATED_AFTER_REFUSAL",
            "At least three contacts followed refusal",
        ),
        (
            input.doxxing_with_toxicity,
            f.doxxing,
            "DOXXING_WITH_TOXICITY",
            "Personal information appears in a toxic context",
        ),
        (
            input.address_with_threat,
            f.address_threat,
            "ADDRESS_WITH_THREAT",
            "An address appears with a threat",
        ),
        (
            input.feed_protection,
            f.pile_on,
            "PILE_ON_FLOOR",
            "A rapid influx of distinct accounts requires feed protection",
        ),
    ] {
        if fired {
            let before = score;
            score = score.max(f64::from(floor));
            reasons.push(reason(code, text, before, score));
        }
    }
    let severity = score.round().clamp(0.0, 100.0) as u8;
    let tier = if severity >= c.thresholds.urgent {
        Tier::Urgent
    } else if severity >= c.thresholds.high {
        Tier::High
    } else if severity >= c.thresholds.review {
        Tier::Review
    } else {
        Tier::Low
    };
    let mut action = match tier {
        Tier::Low => Action::Log,
        Tier::Review => Action::WarnSender,
        Tier::High => Action::Throttle,
        Tier::Urgent => Action::Escalate,
    };
    if input.unsolicited_explicit && tier != Tier::Urgent {
        action = Action::BlurMedia;
    }
    let mut secondary = Vec::new();
    if matches!(tier, Tier::High | Tier::Urgent) {
        secondary.push(SecondaryAction::AlertModerator);
    }
    if tier == Tier::Urgent || input.synthetic_intimate {
        secondary.push(SecondaryAction::NeedsHumanReview);
    }
    if input.feed_protection {
        secondary.extend([SecondaryAction::CollapseReplies, SecondaryAction::RateLimit]);
    }
    degraded.sort();
    degraded.dedup();
    FusionReport {
        severity,
        tier,
        reasons,
        recommended_action: action,
        secondary_actions: secondary,
        simulated,
        degraded,
    }
}
