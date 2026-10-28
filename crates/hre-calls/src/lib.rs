//! Bounded, atomic CSV import and descriptive call features. No storage or risk model.
//! Caller/callee always identify the actual calling/receiving party, regardless of
//! which device exported the log. Call metadata cannot establish silence or intent.

use chrono::{DateTime, FixedOffset, Timelike, Utc};
use hre_core::{Pseudonymizer, Status, SubjectId};
use serde::Serialize;
use std::collections::BTreeSet;
use thiserror::Error;
use uuid::Uuid;

/// Maximum UTF-8 CSV size (2 MiB).
pub const MAX_CSV_BYTES: usize = 2 * 1024 * 1024;
/// Maximum number of call records per import or feature extraction.
pub const MAX_RECORDS: usize = 10_000;
/// Maximum duration of one call, in seconds (24 hours).
pub const MAX_CALL_DURATION_S: u32 = 86_400;
/// Exact required column names; column order may vary.
pub const CSV_COLUMNS: [&str; 9] = [
    "event_id",
    "occurred_at",
    "caller_id",
    "callee_id",
    "duration_s",
    "answered",
    "voicemail",
    "after_refusal",
    "simulated",
];
/// Proposed model vector order. No model is loaded or inferred by this crate.
pub const FEATURE_ORDER: [&str; 8] = [
    "outgoing_calls_1h",
    "outgoing_calls_24h",
    "unanswered_calls_24h",
    "night_calls_24h",
    "after_refusal_calls_24h",
    "answered_duration_s_24h",
    "distinct_callers_24h",
    "recipient_reply_ratio_24h",
];

/// Import/feature errors contain positions and static codes, never CSV values.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CallError {
    /// Input exceeds a fixed resource limit.
    #[error("call input exceeds {0}")]
    Limit(&'static str),
    /// The encoding, header or caller-supplied configuration is invalid.
    #[error("invalid call input: {0}")]
    Invalid(&'static str),
    /// A one-based logical row failed validation (header is row one).
    #[error("invalid call row {row}: {field}")]
    Row { row: usize, field: &'static str },
    /// Pseudonymization failed without exposing the source identity.
    #[error("call identity pseudonymization failed")]
    Identity,
}

/// Validated metadata with HMAC identities; contains no raw CSV or media bodies.
#[derive(Debug, Clone, Serialize)]
pub struct CallRecord {
    /// Client-provided unique event identity.
    pub event_id: Uuid,
    /// UTC call start time.
    pub occurred_at: DateTime<Utc>,
    /// Actual caller, pseudonymized with the supplied platform scope.
    pub caller: SubjectId,
    /// Actual callee, in the same identity scope.
    pub callee: SubjectId,
    /// Exported duration; not an estimate of speech duration.
    pub duration_s: u32,
    /// Whether the recipient answered.
    pub answered: bool,
    /// Exported voicemail indicator, independent of answered state.
    pub voicemail: bool,
    /// Exported refusal state at call time; not inferred from present block state.
    pub after_refusal: bool,
    /// True for synthetic/proxy input; propagated to contributing features.
    pub simulated: bool,
}

/// Parse the entire import or return an error; never return a partial import.
///
/// All nine [`CSV_COLUMNS`] are required. Booleans are exactly `true`/`false`;
/// duration is unsigned decimal; timestamps are RFC3339 in 1970..=2100.
/// Supports UTF-8 BOM, LF/CRLF, quoted commas and doubled quotes. Multiline
/// fields, blank rows, unknown/duplicate columns and duplicate UUIDs are rejected.
/// Limits: 2 MiB, 10,000 records, 4,096 bytes/line, 256 decoded bytes/field.
/// IDs/platform must be nonempty, trimmed and free of control characters.
/// Caller retains input ownership; this function performs no I/O or persistence.
/// Run in `spawn_blocking` when called from async request handlers.
pub fn import_csv(
    input: &[u8],
    pseudonymizer: &Pseudonymizer,
    platform: &str,
) -> Result<Vec<CallRecord>, CallError> {
    if input.len() > MAX_CSV_BYTES {
        return Err(CallError::Limit("csv_bytes"));
    }
    if !valid_identity(platform) {
        return Err(CallError::Invalid("platform"));
    }
    let text = std::str::from_utf8(input).map_err(|_| CallError::Invalid("utf8"))?;
    let mut lines = text.trim_start_matches('\u{feff}').split_terminator('\n');
    let header = parse_row(lines.next().ok_or(CallError::Invalid("header"))?, 1)?;
    if header.len() != CSV_COLUMNS.len() {
        return Err(CallError::Invalid("header"));
    }
    let mut positions = [0; 9];
    for (i, name) in CSV_COLUMNS.iter().enumerate() {
        let found: Vec<_> = header
            .iter()
            .enumerate()
            .filter(|(_, h)| h.as_str() == *name)
            .collect();
        if found.len() != 1 {
            return Err(CallError::Invalid("header"));
        }
        positions[i] = found[0].0;
    }
    let mut records = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let row = index + 2;
        if records.len() >= MAX_RECORDS {
            return Err(CallError::Limit("records"));
        }
        let fields = parse_row(line, row)?;
        if fields.len() != CSV_COLUMNS.len() {
            return Err(CallError::Row {
                row,
                field: "columns",
            });
        }
        let get = |i: usize| fields[positions[i]].as_str();
        let bad = |field| CallError::Row { row, field };
        let event_id = Uuid::parse_str(get(0)).map_err(|_| bad("event_id"))?;
        if event_id.is_nil() || !ids.insert(event_id) {
            return Err(bad("duplicate_or_nil_event_id"));
        }
        let occurred_at = DateTime::parse_from_rfc3339(get(1))
            .map_err(|_| bad("occurred_at"))?
            .with_timezone(&Utc);
        if !valid_time(occurred_at) {
            return Err(bad("occurred_at"));
        }
        for i in [2, 3] {
            if !valid_identity(get(i)) {
                return Err(bad(CSV_COLUMNS[i]));
            }
        }
        if get(2) == get(3) {
            return Err(bad("self_call"));
        }
        if get(4).is_empty() || !get(4).bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad("duration_s"));
        }
        let duration_s = get(4).parse::<u32>().map_err(|_| bad("duration_s"))?;
        if duration_s > MAX_CALL_DURATION_S {
            return Err(bad("duration_s"));
        }
        let boolean = |i| match get(i) {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(bad(CSV_COLUMNS[i])),
        };
        records.push(CallRecord {
            event_id,
            occurred_at,
            caller: pseudonymizer
                .subject(platform, get(2))
                .map_err(|_| CallError::Identity)?,
            callee: pseudonymizer
                .subject(platform, get(3))
                .map_err(|_| CallError::Identity)?,
            duration_s,
            answered: boolean(5)?,
            voicemail: boolean(6)?,
            after_refusal: boolean(7)?,
            simulated: boolean(8)?,
        });
    }
    Ok(records)
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value == value.trim()
        && !value.chars().any(char::is_control)
}

fn valid_time(time: DateTime<Utc>) -> bool {
    use chrono::Datelike;
    (1970..=2100).contains(&time.year()) && time.nanosecond() < 1_000_000_000
}

fn pseudonymous(id: &SubjectId) -> bool {
    id.0.len() == 64
        && id
            .0
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// shortcut: one physical line per record; reject multiline exports until needed.
fn parse_row(line: &str, row: usize) -> Result<Vec<String>, CallError> {
    if line.len() > 4096 {
        return Err(CallError::Limit("line_bytes"));
    }
    let line = line.strip_suffix('\r').unwrap_or(line);
    let bad = || CallError::Row {
        row,
        field: "csv_syntax",
    };
    if line.is_empty() || line.contains(['\r', '\0']) {
        return Err(bad());
    }
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                field.push(ch);
            }
        } else if ch == ',' {
            fields.push(std::mem::take(&mut field));
            closed = false;
            if fields.len() >= CSV_COLUMNS.len() {
                return Err(bad());
            }
        } else if ch == '"' && field.is_empty() && !closed {
            quoted = true;
        } else if ch == '"' || closed {
            return Err(bad());
        } else {
            field.push(ch);
        }
        if field.len() > 256 {
            return Err(CallError::Limit("field_bytes"));
        }
    }
    if quoted {
        return Err(bad());
    }
    fields.push(field);
    Ok(fields)
}

/// Descriptive statistics for one caller->callee window. Empty ratios are zero.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CallWindow {
    /// Calls in the window, including endpoints.
    pub call_count: u32,
    /// Calls whose exported answered flag is false.
    pub unanswered_calls: u32,
    /// Answered calls shorter than five seconds; NOT evidence of acoustic silence.
    pub short_answered_calls: u32,
    /// Calls during 22:00..05:00 at the supplied recipient UTC offset.
    pub night_calls: u32,
    /// Calls whose supplied after_refusal flag is true.
    pub after_refusal_calls: u32,
    /// Calls marked as voicemail by the exporter.
    pub voicemail_count: u32,
    /// Sum of duration for answered calls only, in seconds.
    pub answered_duration_s: u64,
    /// Mean exported duration of all calls, in seconds.
    pub avg_duration_s: f64,
    /// Unanswered calls / all calls.
    pub unanswered_ratio: f64,
    /// Night calls / all calls.
    pub night_call_ratio: f64,
}

/// Pair features; descriptive output is Built/Simulated, with no risk score.
#[derive(Debug, Clone, Serialize)]
pub struct CallFeatures {
    /// Whether any selected pair/reverse/target-window input was simulated.
    pub simulated: bool,
    /// Built for real metadata, Simulated when contributing data is synthetic.
    pub status: Status,
    /// One-hour directional statistics.
    pub one_hour: CallWindow,
    /// Twenty-four-hour directional statistics.
    pub one_day: CallWindow,
    /// Seven-day directional statistics.
    pub seven_days: CallWindow,
    /// Largest inclusive rolling one-hour count within the directional seven days.
    pub max_calls_1h: u32,
    /// Population standard deviation / mean of directional seven-day gaps;
    /// None for fewer than two gaps or zero mean (all timestamps identical).
    pub gap_regularity: Option<f64>,
    /// Distinct actual callers to this callee during the last 24 hours.
    pub distinct_callers_24h: u32,
    /// Reverse calls / (forward + reverse calls) during the last 24 hours.
    /// An answered call does not imply reciprocity. Zero for no calls.
    pub recipient_reply_ratio_24h: f64,
    /// Always None: metadata does not identify a new number or link identities.
    pub new_number_flag: Option<bool>,
}

impl CallFeatures {
    /// Raw, unnormalized values in [`FEATURE_ORDER`], not a trained model contract.
    pub fn model_vector(&self) -> [f64; 8] {
        [
            self.one_hour.call_count as f64,
            self.one_day.call_count as f64,
            self.one_day.unanswered_calls as f64,
            self.one_day.night_calls as f64,
            self.one_day.after_refusal_calls as f64,
            self.one_day.answered_duration_s as f64,
            self.distinct_callers_24h as f64,
            self.recipient_reply_ratio_24h,
        ]
    }
}

/// Extract independent directional 1h/24h/7d windows ending at `as_of` (inclusive).
/// Excludes future/out-of-window calls and sorts a borrowed view of input.
/// `recipient_offset` defaults at the API boundary to +05:30 for Kolkata; callers
/// must supply the actual recipient offset (DST zones need caller-side conversion).
/// Rejects oversized input, duplicate/nil UUIDs, invalid time/duration or non-HMAC
/// shaped IDs. This validates ID shape, not HMAC authenticity or history completeness.
/// Input must cover all calls to the target for distinct-caller counts; no missing
/// history is inferred. No persistence, alerts, or model inference takes place.
pub fn extract_features(
    records: &[CallRecord],
    caller: &SubjectId,
    callee: &SubjectId,
    as_of: DateTime<Utc>,
    recipient_offset: FixedOffset,
) -> Result<CallFeatures, CallError> {
    if records.len() > MAX_RECORDS {
        return Err(CallError::Limit("records"));
    }
    if !pseudonymous(caller) || !pseudonymous(callee) || caller == callee || !valid_time(as_of) {
        return Err(CallError::Invalid("pair_or_as_of"));
    }
    let mut ids = BTreeSet::new();
    for r in records {
        if r.event_id.is_nil()
            || !ids.insert(r.event_id)
            || !valid_time(r.occurred_at)
            || r.duration_s > MAX_CALL_DURATION_S
            || !pseudonymous(&r.caller)
            || !pseudonymous(&r.callee)
            || r.caller == r.callee
        {
            return Err(CallError::Invalid("record"));
        }
    }
    let within = |r: &CallRecord, seconds| {
        let age = as_of.signed_duration_since(r.occurred_at);
        age >= chrono::Duration::zero() && age <= chrono::Duration::seconds(seconds)
    };
    let mut forward: Vec<_> = records
        .iter()
        .filter(|r| r.caller == *caller && r.callee == *callee && within(r, 604_800))
        .collect();
    forward.sort_by_key(|r| (r.occurred_at, r.event_id));
    let summarize = |seconds| {
        let mut window = CallWindow::default();
        let mut total_duration = 0u64;
        for r in forward.iter().filter(|r| within(r, seconds)) {
            window.call_count += 1;
            window.unanswered_calls += u32::from(!r.answered);
            window.short_answered_calls += u32::from(r.answered && r.duration_s < 5);
            let hour = r.occurred_at.with_timezone(&recipient_offset).hour();
            window.night_calls += u32::from(!(5..22).contains(&hour));
            window.after_refusal_calls += u32::from(r.after_refusal);
            window.voicemail_count += u32::from(r.voicemail);
            window.answered_duration_s += if r.answered {
                u64::from(r.duration_s)
            } else {
                0
            };
            total_duration += u64::from(r.duration_s);
        }
        if window.call_count > 0 {
            let count = f64::from(window.call_count);
            window.avg_duration_s = total_duration as f64 / count;
            window.unanswered_ratio = f64::from(window.unanswered_calls) / count;
            window.night_call_ratio = f64::from(window.night_calls) / count;
        }
        window
    };
    let one_day = summarize(86_400);
    let reverse_count = records
        .iter()
        .filter(|r| r.caller == *callee && r.callee == *caller && within(r, 86_400))
        .count();
    let target: Vec<_> = records
        .iter()
        .filter(|r| r.callee == *callee && within(r, 86_400))
        .collect();
    let distinct_callers_24h = target
        .iter()
        .map(|r| &r.caller)
        .collect::<BTreeSet<_>>()
        .len() as u32;
    let simulated = forward.iter().any(|r| r.simulated)
        || target.iter().any(|r| r.simulated)
        || records.iter().any(|r| {
            r.caller == *callee && r.callee == *caller && within(r, 86_400) && r.simulated
        });
    let total = f64::from(one_day.call_count) + reverse_count as f64;
    let mut max_calls_1h = 0;
    let mut left = 0;
    for right in 0..forward.len() {
        while forward[right]
            .occurred_at
            .signed_duration_since(forward[left].occurred_at)
            > chrono::Duration::hours(1)
        {
            left += 1;
        }
        max_calls_1h = max_calls_1h.max((right - left + 1) as u32);
    }
    let gaps: Vec<_> = forward
        .windows(2)
        .map(|w| {
            w[1].occurred_at
                .signed_duration_since(w[0].occurred_at)
                .num_nanoseconds()
                .unwrap_or_default() as f64
                / 1e9
        })
        .collect();
    let gap_regularity = if gaps.len() >= 2 {
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        (mean > 0.0).then(|| {
            (gaps.iter().map(|gap| (gap - mean).powi(2)).sum::<f64>() / gaps.len() as f64).sqrt()
                / mean
        })
    } else {
        None
    };
    Ok(CallFeatures {
        simulated,
        status: if simulated {
            Status::Simulated
        } else {
            Status::Built
        },
        one_hour: summarize(3600),
        seven_days: summarize(604_800),
        one_day,
        max_calls_1h,
        gap_regularity,
        distinct_callers_24h,
        recipient_reply_ratio_24h: if total > 0.0 {
            reverse_count as f64 / total
        } else {
            0.0
        },
        new_number_flag: None,
    })
}

#[cfg(test)]
mod tests;
