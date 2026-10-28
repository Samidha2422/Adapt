use crate::csv::CallRecord;
use hre_core::CallDirection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallFeatures {
    pub call_count_1h: usize,
    pub call_count_24h: usize,
    pub call_count_7d: usize,
    pub max_calls_1h: usize,
    pub unanswered_ratio_24h: f64,
    pub silent_calls_24h: usize,
    pub avg_duration_s_24h: f64,
    pub night_call_ratio_24h: f64,
    pub after_block_calls: usize,
    pub new_number_flag: bool,
    pub gap_regularity: f64,
    pub voicemail_count_24h: usize,
}

#[derive(Debug, Clone)]
pub struct CallFeatureConfig {
    pub night_start_hour: u32,
    pub night_end_hour: u32,
    pub utc_offset_minutes: i32,
}

impl Default for CallFeatureConfig {
    fn default() -> Self {
        Self {
            night_start_hour: 22,
            night_end_hour: 6,
            utc_offset_minutes: 330, // Asia/Kolkata (+05:30)
        }
    }
}

/// Computes the 10 call features for calls from `caller` to `callee` up to `eval_time_ms`.
pub fn extract_call_features(
    history: &[CallRecord],
    caller: &str,
    callee: &str,
    eval_time_ms: i64,
    config: &CallFeatureConfig,
) -> CallFeatures {
    let mut relevant: Vec<&CallRecord> = history
        .iter()
        .filter(|r| {
            r.caller == caller
                && r.callee == callee
                && r.direction == CallDirection::Incoming
                && r.timestamp_ms <= eval_time_ms
        })
        .collect();

    relevant.sort_by_key(|r| r.timestamp_ms);

    let ms_1h = 3600 * 1000;
    let ms_24h = 24 * 3600 * 1000;
    let ms_7d = 7 * ms_24h;

    let calls_1h: Vec<&&CallRecord> = relevant
        .iter()
        .filter(|r| r.timestamp_ms >= eval_time_ms.saturating_sub(ms_1h))
        .collect();
    let calls_24h: Vec<&&CallRecord> = relevant
        .iter()
        .filter(|r| r.timestamp_ms >= eval_time_ms.saturating_sub(ms_24h))
        .collect();
    let calls_7d: Vec<&&CallRecord> = relevant
        .iter()
        .filter(|r| r.timestamp_ms >= eval_time_ms.saturating_sub(ms_7d))
        .collect();

    let call_count_1h = calls_1h.len();
    let call_count_24h = calls_24h.len();
    let call_count_7d = calls_7d.len();

    // Max calls in any 1-hour window in the last 24h
    let mut max_calls_1h = call_count_1h;
    for (i, r1) in calls_24h.iter().enumerate() {
        let count = calls_24h[i..]
            .iter()
            .take_while(|r2| r2.timestamp_ms <= r1.timestamp_ms + ms_1h)
            .count();
        if count > max_calls_1h {
            max_calls_1h = count;
        }
    }

    // 24h metrics
    let (unanswered, silent, total_duration, night_calls, voicemails) = calls_24h.iter().fold(
        (0usize, 0usize, 0.0f64, 0usize, 0usize),
        |(unans, sil, dur, night, vm), r| {
            let is_unans = if !r.answered { 1 } else { 0 };
            let is_sil = if r.answered && r.duration_s < 5 { 1 } else { 0 };
            let is_night = if is_night_time(r.timestamp_ms, config) {
                1
            } else {
                0
            };
            let is_vm = if r.voicemail { 1 } else { 0 };
            (
                unans + is_unans,
                sil + is_sil,
                dur + r.duration_s as f64,
                night + is_night,
                vm + is_vm,
            )
        },
    );

    let unanswered_ratio_24h = if call_count_24h > 0 {
        unanswered as f64 / call_count_24h as f64
    } else {
        0.0
    };

    let avg_duration_s_24h = if call_count_24h > 0 {
        total_duration / call_count_24h as f64
    } else {
        0.0
    };

    let night_call_ratio_24h = if call_count_24h > 0 {
        night_calls as f64 / call_count_24h as f64
    } else {
        0.0
    };

    // Gap regularity (std / mean of inter-call gaps in 24h)
    let gap_regularity = if calls_24h.len() >= 2 {
        let mut gaps = Vec::with_capacity(calls_24h.len() - 1);
        for w in calls_24h.windows(2) {
            let gap_s = (w[1].timestamp_ms.saturating_sub(w[0].timestamp_ms) / 1000) as f64;
            gaps.push(gap_s);
        }
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        if mean > 0.0 {
            let variance = gaps.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / gaps.len() as f64;
            variance.sqrt() / mean
        } else {
            0.0
        }
    } else {
        0.0
    };

    let new_number_flag = call_count_7d <= 3 && call_count_24h >= 2;

    CallFeatures {
        call_count_1h,
        call_count_24h,
        call_count_7d,
        max_calls_1h,
        unanswered_ratio_24h,
        silent_calls_24h: silent,
        avg_duration_s_24h,
        night_call_ratio_24h,
        after_block_calls: 0, // Injected from pair refusal state
        new_number_flag,
        gap_regularity,
        voicemail_count_24h: voicemails,
    }
}

fn is_night_time(timestamp_ms: i64, config: &CallFeatureConfig) -> bool {
    let shifted_s = (timestamp_ms / 1000) + (config.utc_offset_minutes as i64 * 60);
    let seconds_in_day = shifted_s.rem_euclid(86400);
    let hour = (seconds_in_day / 3600) as u32;

    if config.night_start_hour > config.night_end_hour {
        hour >= config.night_start_hour || hour < config.night_end_hour
    } else {
        hour >= config.night_start_hour && hour < config.night_end_hour
    }
}
