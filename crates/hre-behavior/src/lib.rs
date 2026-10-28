//! Deterministic directional features, protective context, pile-on and occlusion attribution.
use hre_core::{Backend, Reason, Signal, Status};
use hre_ml::{InferenceBackend, LogisticFallback};
use hre_store::{EventKind, StoredEvent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

const DAY: i64 = 86_400_000;
/// Stable feature vector schema, expanded window-major by `feature_order`.
pub const WINDOW_FEATURES: &[&str] = &[
    "msg_count",
    "reply_ratio",
    "night_ratio",
    "tox_mean",
    "tox_max",
    "threat_max",
    "sexual_max",
    "sent_mean",
    "pii_mentions",
    "location_cue_count",
    "unsolicited_media_count",
];
pub const GLOBAL_FEATURES: &[&str] = &[
    "unreplied_streak",
    "initiator_ratio",
    "median_gap_s",
    "burst_max_10m",
    "after_refusal_count",
    "since_refusal_hours",
    "active_days_streak",
    "tox_slope_last20",
    "sent_slope_last20",
    "repetition_score",
    "sender_account_age_days",
    "is_contact",
    "distinct_channels",
    "evasion_similarity",
];
/// Exact model/training order. Missing account age is -1; evasion is unavailable and zero.
pub fn feature_order() -> Vec<String> {
    ["1h", "24h", "7d"]
        .iter()
        .flat_map(|w| WINDOW_FEATURES.iter().map(move |f| format!("{f}_{w}")))
        .chain(GLOBAL_FEATURES.iter().map(|s| (*s).into()))
        .collect()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BehaviorConfig {
    pub unreplied_review_count: usize,
    pub unreplied_high_count: usize,
    pub pile_on_senders: usize,
    pub pile_on_window_seconds: i64,
    pub reciprocity_protective_ratio: f64,
    pub reverse_harassment_guard_count: usize,
    pub recipient_utc_offset_minutes: i32,
    pub night_start_hour: u32,
    pub night_end_hour: u32,
    pub logistic_bias: f64,
    pub logistic_weights: BTreeMap<String, f64>,
}
impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            unreplied_review_count: 20,
            unreplied_high_count: 60,
            pile_on_senders: 10,
            pile_on_window_seconds: 60,
            reciprocity_protective_ratio: 0.4,
            reverse_harassment_guard_count: 20,
            recipient_utc_offset_minutes: 0,
            night_start_hour: 22,
            night_end_hour: 6,
            logistic_bias: -4.0,
            logistic_weights: BTreeMap::from([
                ("unreplied_streak".into(), 4.0),
                ("after_refusal_count".into(), 3.0),
                ("tox_slope_last20".into(), 2.0),
                ("reply_ratio_24h".into(), -4.0),
                ("night_ratio_24h".into(), 0.4),
            ]),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum BehaviorError {
    #[error("behavior TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid behavior configuration")]
    InvalidConfig,
}
impl BehaviorConfig {
    pub fn parse(toml: &str) -> Result<Self, BehaviorError> {
        let c: Self = toml::from_str(toml)?;
        c.validate()?;
        Ok(c)
    }
    pub fn validate(&self) -> Result<(), BehaviorError> {
        let order = feature_order();
        if self.unreplied_review_count == 0
            || self.unreplied_high_count < self.unreplied_review_count
            || self.pile_on_senders < 2
            || !(1..=3600).contains(&self.pile_on_window_seconds)
            || self.reverse_harassment_guard_count == 0
            || self.night_start_hour > 23
            || self.night_end_hour > 23
            || !(-1440..=1440).contains(&self.recipient_utc_offset_minutes)
            || !self.reciprocity_protective_ratio.is_finite()
            || !(0.0..=1.0).contains(&self.reciprocity_protective_ratio)
            || !self.logistic_bias.is_finite()
            || self
                .logistic_weights
                .iter()
                .any(|(k, v)| !order.contains(k) || !v.is_finite())
        {
            return Err(BehaviorError::InvalidConfig);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Features {
    pub values: BTreeMap<String, f64>,
}
impl Features {
    pub fn get(&self, name: &str) -> f64 {
        self.values.get(name).copied().unwrap_or(0.0)
    }
    pub fn vector(&self) -> Vec<f64> {
        feature_order().iter().map(|k| self.get(k)).collect()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorReport {
    pub features: Features,
    pub behavior: Signal,
    pub pile_on: Signal,
    pub reasons: Vec<Reason>,
    pub reply_to_harasser: bool,
    pub healthy_reciprocity: bool,
    pub recipient_initiated: bool,
    pub after_refusal_count: usize,
    pub feed_protection: bool,
    pub simulated: bool,
    pub degraded: Vec<String>,
}
fn reason(code: &str, text: String, weight: f64) -> Reason {
    Reason {
        code: code.into(),
        text,
        weight,
        evidence: BTreeMap::new(),
    }
}
fn contact(e: &StoredEvent) -> bool {
    e.kind != EventKind::Refusal
}
fn message(e: &StoredEvent) -> bool {
    e.kind == EventKind::Message
}
fn recent(e: &StoredEvent, now: i64, window: i64) -> bool {
    e.timestamp_ms >= now.saturating_sub(window) && e.timestamp_ms <= now
}
fn night(e: &StoredEvent, c: &BehaviorConfig) -> bool {
    let hour = (e.timestamp_ms.div_euclid(60_000) + i64::from(c.recipient_utc_offset_minutes))
        .rem_euclid(1440)
        / 60;
    let (start, end) = (i64::from(c.night_start_hour), i64::from(c.night_end_hour));
    if start <= end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}
fn ratio(n: usize, d: usize) -> f64 {
    if d == 0 {
        0.0
    } else {
        n as f64 / d as f64
    }
}
fn slope(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mx = (values.len() - 1) as f64 / 2.0;
    let my = values.iter().sum::<f64>() / values.len() as f64;
    values
        .iter()
        .enumerate()
        .map(|(i, y)| (i as f64 - mx) * (y - my))
        .sum::<f64>()
        / (0..values.len())
            .map(|i| (i as f64 - mx).powi(2))
            .sum::<f64>()
}
fn streak(pair: &[&StoredEvent], sender: &str) -> usize {
    pair.iter().rev().take_while(|e| e.sender == sender).count()
}
/// Extract from seven days of contacts plus latest refusal markers of any age in both directions.
/// Input is sorted and deduplicated by event ID. Future records never contribute.
pub fn extract_features(
    events: &[StoredEvent],
    sender: &str,
    recipient: &str,
    now_ms: i64,
    c: &BehaviorConfig,
) -> Features {
    let mut seen = BTreeSet::new();
    let mut pair: Vec<_> = events
        .iter()
        .filter(|e| {
            (recent(e, now_ms, 7 * DAY)
                || (e.kind == EventKind::Refusal && e.timestamp_ms <= now_ms))
                && ((e.sender == sender && e.recipient == recipient)
                    || (e.sender == recipient && e.recipient == sender))
                && seen.insert(&e.event_id)
        })
        .collect();
    pair.sort_by(|a, b| (a.timestamp_ms, &a.event_id).cmp(&(b.timestamp_ms, &b.event_id)));
    let out: Vec<_> = pair
        .iter()
        .copied()
        .filter(|e| e.sender == sender && message(e))
        .collect();
    let contacts: Vec<_> = pair.iter().copied().filter(|e| contact(e)).collect();
    let mut f = Features::default();
    for (w, ms) in [("1h", 3_600_000), ("24h", DAY), ("7d", 7 * DAY)] {
        let current: Vec<_> = out
            .iter()
            .copied()
            .filter(|e| recent(e, now_ms, ms))
            .collect();
        let replies = pair
            .iter()
            .filter(|e| e.sender == recipient && message(e) && recent(e, now_ms, ms))
            .count();
        let n = current.len();
        let mean = |pick: fn(&StoredEvent) -> f64| {
            if n == 0 {
                0.0
            } else {
                current.iter().map(|e| pick(e)).sum::<f64>() / n as f64
            }
        };
        let max =
            |pick: fn(&StoredEvent) -> f64| current.iter().map(|e| pick(e)).fold(0.0, f64::max);
        let values = [
            n as f64,
            ratio(replies, n + replies),
            ratio(current.iter().filter(|e| night(e, c)).count(), n),
            mean(|e| e.toxicity),
            max(|e| e.toxicity),
            max(|e| e.threat),
            max(|e| e.sexual),
            mean(|e| e.sentiment),
            current.iter().map(|e| f64::from(e.pii_mentions)).sum(),
            current.iter().map(|e| f64::from(e.location_cues)).sum(),
            current.iter().filter(|e| e.unsolicited_media).count() as f64,
        ];
        for (name, value) in WINDOW_FEATURES.iter().zip(values) {
            f.values.insert(format!("{name}_{w}"), value);
        }
        // Media is counted over all contact kinds rather than message-only rows.
        let media = pair
            .iter()
            .filter(|e| {
                e.sender == sender && contact(e) && e.unsolicited_media && recent(e, now_ms, ms)
            })
            .count();
        f.values
            .insert(format!("unsolicited_media_count_{w}"), media as f64);
    }
    let mut gaps: Vec<f64> = out
        .windows(2)
        .map(|p| p[1].timestamp_ms.saturating_sub(p[0].timestamp_ms) as f64 / 1000.0)
        .collect();
    gaps.sort_by(f64::total_cmp);
    let median = if gaps.is_empty() {
        0.0
    } else {
        (gaps[(gaps.len() - 1) / 2] + gaps[gaps.len() / 2]) / 2.0
    };
    let mut left = 0;
    let mut burst = 0;
    for right in 0..out.len() {
        while out[right]
            .timestamp_ms
            .saturating_sub(out[left].timestamp_ms)
            > 600_000
        {
            left += 1;
        }
        burst = burst.max(right - left + 1);
    }
    let markers: Vec<_> = pair
        .iter()
        .filter(|e| e.sender == sender && e.kind == EventKind::Refusal)
        .copied()
        .collect();
    let block = markers
        .iter()
        .rev()
        .find(|e| e.channel == "block" || e.channel == "unblock")
        .copied();
    let unblock = block.filter(|e| e.channel == "unblock");
    let active = markers
        .iter()
        .copied()
        .filter(|e| {
            e.channel != "block" && e.channel != "unblock"
                || block.is_some_and(|b| b.channel == "block" && b.event_id == e.event_id)
        })
        .map(|e| e.timestamp_ms)
        .min();
    let after = contacts
        .iter()
        .filter(|e| {
            e.sender == sender
                && match active {
                    Some(t) => e.timestamp_ms > t || (e.timestamp_ms == t && e.after_refusal),
                    None => {
                        e.after_refusal && unblock.is_none_or(|m| e.timestamp_ms > m.timestamp_ms)
                    }
                }
        })
        .count();
    let days: Vec<_> = out
        .iter()
        .map(|e| e.timestamp_ms.div_euclid(DAY))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .rev()
        .collect();
    let mut active_days = usize::from(!days.is_empty());
    for d in days.windows(2) {
        if d[0] - d[1] != 1 {
            break;
        }
        active_days += 1;
    }
    let last20: Vec<_> = out.iter().rev().take(20).rev().copied().collect();
    let mut hashes = BTreeMap::<&str, usize>::new();
    for h in out.iter().filter_map(|e| e.content_hash.as_deref()) {
        *hashes.entry(h).or_default() += 1;
    }
    let known: usize = hashes.values().sum();
    let repeat = if known < 2 {
        0.0
    } else {
        hashes.values().map(|n| n * (n - 1)).sum::<usize>() as f64 / (known * (known - 1)) as f64
    };
    let initiator = contacts.first().is_some_and(|e| e.sender == sender);
    let global = [
        streak(&contacts, sender) as f64,
        u8::from(initiator) as f64,
        median,
        burst as f64,
        after as f64,
        active.map_or(-1.0, |t| now_ms.saturating_sub(t) as f64 / 3_600_000.0),
        active_days as f64,
        slope(&last20.iter().map(|e| e.toxicity).collect::<Vec<_>>()),
        slope(&last20.iter().map(|e| e.sentiment).collect::<Vec<_>>()),
        repeat,
        out.last()
            .and_then(|e| e.sender_account_age_days)
            .unwrap_or(-1.0),
        u8::from(out.last().is_some_and(|e| e.is_contact)) as f64,
        contacts
            .iter()
            .map(|e| &e.channel)
            .collect::<BTreeSet<_>>()
            .len() as f64,
        0.0,
    ];
    for (name, value) in GLOBAL_FEATURES.iter().zip(global) {
        f.values.insert((*name).into(), value);
    }
    f
}
fn normalized(f: &Features, key: &str, c: &BehaviorConfig) -> f64 {
    let value = f.get(key);
    match key {
        "unreplied_streak" => (value / c.unreplied_review_count.max(1) as f64).min(4.0),
        "after_refusal_count" => value.min(3.0),
        _ => value,
    }
}
fn probability(f: &Features, c: &BehaviorConfig) -> f64 {
    let backend = LogisticFallback {
        bias: c.logistic_bias,
        weights: c.logistic_weights.values().copied().collect(),
    };
    backend
        .predict(
            &c.logistic_weights
                .keys()
                .map(|k| normalized(f, k, c))
                .collect::<Vec<_>>(),
        )
        .unwrap_or(0.0)
}
/// Positive probability deltas when replacing one feature with the benign baseline (zero).
/// Missing/refusal/account-age sentinel features retain -1 as their baseline.
pub fn occlusion_attribution(f: &Features, c: &BehaviorConfig, top_k: usize) -> Vec<Reason> {
    let full = probability(f, c);
    let mut deltas: Vec<_> = c
        .logistic_weights
        .keys()
        .filter_map(|key| {
            let mut baseline = f.clone();
            baseline.values.insert(
                key.clone(),
                if key == "sender_account_age_days" || key == "since_refusal_hours" {
                    -1.0
                } else {
                    0.0
                },
            );
            let delta = full - probability(&baseline, c);
            (delta > 0.0001).then(|| {
                let mut r = reason(
                    "BEHAVIOR_OCCLUSION",
                    format!("{key}: {:.3}", f.get(key)),
                    delta,
                );
                r.evidence.insert("feature".into(), serde_json::json!(key));
                r.evidence
                    .insert("value".into(), serde_json::json!(f.get(key)));
                r
            })
        })
        .collect();
    deltas.sort_by(|a, b| {
        b.weight
            .total_cmp(&a.weight)
            .then_with(|| a.text.cmp(&b.text))
    });
    deltas.truncate(top_k);
    deltas
}
/// Analyze both directional histories plus target-wide events with an optional inference backend.
pub fn analyze_with_backend(
    events: &[StoredEvent],
    sender: &str,
    recipient: &str,
    now_ms: i64,
    config: &BehaviorConfig,
    backend: Option<&dyn InferenceBackend>,
) -> BehaviorReport {
    // Invalid programmatic configs degrade to the documented safe defaults, never panic.
    let defaults = BehaviorConfig::default();
    let c = if config.validate().is_ok() {
        config
    } else {
        &defaults
    };
    let features = extract_features(events, sender, recipient, now_ms, c);
    // The pending reply must not erase the unreplied streak that preceded it.
    let pending = events
        .iter()
        .filter(|e| {
            e.sender == sender && e.recipient == recipient && contact(e) && e.timestamp_ms <= now_ms
        })
        .max_by(|a, b| (a.timestamp_ms, &a.event_id).cmp(&(b.timestamp_ms, &b.event_id)))
        .map(|e| &e.event_id);
    let reverse_events: Vec<_> = events
        .iter()
        .filter(|e| {
            (recent(e, now_ms, DAY) || (e.kind == EventKind::Refusal && e.timestamp_ms <= now_ms))
                && Some(&e.event_id) != pending
        })
        .cloned()
        .collect();
    let reverse = extract_features(&reverse_events, recipient, sender, now_ms, c);
    let reply_to_harasser = reverse.get("unreplied_streak")
        >= c.reverse_harassment_guard_count as f64
        || reverse.get("after_refusal_count") > 0.0
        || (reverse.get("msg_count_24h") >= c.reverse_harassment_guard_count as f64
            && reverse.get("tox_mean_24h") >= 0.6);
    let after = features.get("after_refusal_count") as usize;
    let recipient_initiated = events.iter().any(|e| {
        e.sender == sender
            && e.recipient == recipient
            && recent(e, now_ms, 7 * DAY)
            && e.recipient_initiated
    }) || features.get("initiator_ratio") == 0.0;
    let mut call_ids = BTreeSet::new();
    let calls: Vec<_> = events
        .iter()
        .filter(|e| {
            e.sender == sender
                && e.recipient == recipient
                && e.kind == EventKind::Call
                && recent(e, now_ms, DAY)
                && call_ids.insert(&e.event_id)
        })
        .collect();
    let answered: Vec<_> = calls.iter().filter(|e| e.answered).collect();
    let long_answered_calls = !answered.is_empty()
        && ratio(answered.len(), calls.len()) >= 0.8
        && answered.iter().map(|e| e.duration_s).sum::<f64>() / answered.len() as f64 >= 60.0
        && calls.iter().any(|e| e.is_contact);
    let healthy = ((features.get("reply_ratio_24h") > c.reciprocity_protective_ratio
        && recipient_initiated)
        || long_answered_calls)
        && after == 0
        && features.get("since_refusal_hours") < 0.0;
    let mut onnx_active = false;
    let base_prob = if let Some(b) = backend {
        match b.predict(&features.vector()) {
            Ok(p) => {
                if b.backend() == Backend::Onnx {
                    onnx_active = true;
                }
                p
            }
            Err(_) => probability(&features, c),
        }
    } else {
        probability(&features, c)
    };
    let mut score = base_prob;
    let streak = features.get("unreplied_streak") as usize;
    if streak >= c.unreplied_high_count {
        score = score.max(0.85);
    } else if streak >= c.unreplied_review_count {
        score = score.max(0.45);
    }
    if after > 0 {
        score = score.max(if after >= 3 { 0.9 } else { 0.75 });
    }
    if healthy {
        score = score.min(0.1);
    }
    if reply_to_harasser {
        score = 0.0;
    }
    let mut seen = BTreeSet::new();
    let target: Vec<_> = events
        .iter()
        .filter(|e| {
            e.recipient == recipient
                && e.sender != recipient
                && contact(e)
                && recent(e, now_ms, c.pile_on_window_seconds * 1000)
                && seen.insert(&e.event_id)
        })
        .collect();
    let accounts = target
        .iter()
        .map(|e| &e.sender)
        .collect::<BTreeSet<_>>()
        .len();
    let feed_protection = accounts >= c.pile_on_senders;
    let simulated = events
        .iter()
        .any(|e| e.simulated && recent(e, now_ms, 7 * DAY));
    let mut behavior = Signal {
        score,
        label: if onnx_active {
            "onnx_behavior_model".into()
        } else {
            "deterministic_behavior_rules".into()
        },
        status: if simulated {
            Status::Simulated
        } else {
            Status::Built
        },
        backend: if onnx_active {
            Backend::Onnx
        } else {
            Backend::Heuristic
        },
        confidence: if onnx_active { 0.85 } else { 0.6 },
        details: BTreeMap::new(),
    };
    behavior
        .details
        .insert("features".into(), serde_json::json!(features));
    let mut pile_on = Signal {
        score: if feed_protection { 0.85 } else { 0.0 },
        label: "distinct_senders_window".into(),
        status: if simulated {
            Status::Simulated
        } else {
            Status::Built
        },
        backend: Backend::Heuristic,
        confidence: 0.6,
        details: BTreeMap::new(),
    };
    pile_on
        .details
        .insert("distinct_senders".into(), serde_json::json!(accounts));
    pile_on.details.insert(
        "window_seconds".into(),
        serde_json::json!(c.pile_on_window_seconds),
    );
    let mut reasons = occlusion_attribution(&features, c, 3);
    if streak >= c.unreplied_review_count && !healthy {
        reasons.push(reason(
            "PERSISTENT_CONTACT",
            format!("{streak} consecutive contacts without a reply"),
            score,
        ));
    }
    if after > 0 {
        reasons.push(reason(
            "CONTACT_AFTER_REFUSAL",
            format!("{after} contacts after refusal"),
            score,
        ));
    }
    if reply_to_harasser {
        reasons.clear();
        reasons.push(reason(
            "REPLY_TO_HARASSER",
            "Reply is directed at an account with a recent harassment pattern".into(),
            0.0,
        ));
    }
    if healthy {
        reasons.push(reason(
            "HEALTHY_RECIPROCITY",
            "Reciprocal messages or long answered calls reduce behavioral risk".into(),
            -0.1,
        ));
    }
    if feed_protection {
        reasons.push(reason(
            "PILE_ON",
            format!(
                "{accounts} accounts contacted this recipient within {} seconds",
                c.pile_on_window_seconds
            ),
            0.85,
        ));
    }
    let mut degraded = Vec::new();
    if !onnx_active {
        degraded.push("behavior_msg.onnx_missing".into());
    }
    degraded.push("behavior.evasion_similarity_unavailable".into());
    degraded.push("pile_on.ewma_baseline_unavailable".into());
    if config.validate().is_err() {
        degraded.push("behavior.invalid_config_defaults_used".into());
    }
    BehaviorReport {
        features,
        behavior,
        pile_on,
        reasons,
        reply_to_harasser,
        healthy_reciprocity: healthy,
        recipient_initiated,
        after_refusal_count: after,
        feed_protection,
        simulated,
        degraded,
    }
}

/// Analyze both directional histories plus target-wide events. Pass the pending event exactly once.
pub fn analyze(
    events: &[StoredEvent],
    sender: &str,
    recipient: &str,
    now_ms: i64,
    config: &BehaviorConfig,
) -> BehaviorReport {
    analyze_with_backend(events, sender, recipient, now_ms, config, None)
}
