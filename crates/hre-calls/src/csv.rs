use hre_core::CallDirection;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CallsError {
    #[error("empty CSV content")]
    EmptyInput,
    #[error("CSV line exceeds maximum length: line {0}")]
    LineTooLong(usize),
    #[error("too many records in import: {0} > {1}")]
    TooManyRecords(usize, usize),
    #[error("missing required column: {0}")]
    MissingColumn(&'static str),
    #[error("invalid CSV record at line {line}: {detail}")]
    InvalidRecord { line: usize, detail: String },
}

pub const MAX_CSV_LINE_LEN: usize = 2048;
pub const MAX_IMPORT_RECORDS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallRecord {
    pub timestamp_ms: i64,
    pub caller: String,
    pub callee: String,
    pub duration_s: u32,
    pub answered: bool,
    pub direction: CallDirection,
    pub voicemail: bool,
}

/// Parses a CSV string of call logs into strongly-typed `CallRecord` entries.
///
/// Supported headers (case-insensitive):
/// - `timestamp` or `occurred_at` (UNIX seconds/milliseconds or ISO 8601)
/// - `caller` or `from` or `sender`
/// - `callee` or `to` or `recipient`
/// - `duration_s` or `duration`
/// - `answered` or `status` (true/false, 1/0, yes/no, answered/missed)
/// - `direction` (incoming/outgoing)
/// - `voicemail` (optional bool)
pub fn parse_call_log_csv(csv_content: &str) -> Result<Vec<CallRecord>, CallsError> {
    let trimmed = csv_content.trim();
    if trimmed.is_empty() {
        return Err(CallsError::EmptyInput);
    }

    let mut lines = trimmed.lines().enumerate();
    let (_, header_line) = lines.next().ok_or(CallsError::EmptyInput)?;
    if header_line.len() > MAX_CSV_LINE_LEN {
        return Err(CallsError::LineTooLong(1));
    }

    let headers: Vec<String> = header_line
        .split(',')
        .map(|h| h.trim().to_lowercase())
        .collect();

    let ts_idx = headers
        .iter()
        .position(|h| h == "timestamp" || h == "occurred_at" || h == "time" || h == "date")
        .ok_or(CallsError::MissingColumn("timestamp"))?;
    let caller_idx = headers
        .iter()
        .position(|h| h == "caller" || h == "from" || h == "sender" || h == "caller_id")
        .ok_or(CallsError::MissingColumn("caller"))?;
    let callee_idx = headers
        .iter()
        .position(|h| h == "callee" || h == "to" || h == "recipient" || h == "recipient_id")
        .ok_or(CallsError::MissingColumn("callee"))?;
    let duration_idx = headers
        .iter()
        .position(|h| h == "duration_s" || h == "duration" || h == "seconds");
    let answered_idx = headers
        .iter()
        .position(|h| h == "answered" || h == "status" || h == "is_answered");
    let direction_idx = headers
        .iter()
        .position(|h| h == "direction" || h == "call_direction");
    let voicemail_idx = headers
        .iter()
        .position(|h| h == "voicemail" || h == "is_voicemail");

    let mut records = Vec::new();

    for (line_num_zero, line) in lines {
        let line_num = line_num_zero + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() > MAX_CSV_LINE_LEN {
            return Err(CallsError::LineTooLong(line_num));
        }

        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        let max_idx = ts_idx.max(caller_idx).max(callee_idx);
        if fields.len() <= max_idx {
            return Err(CallsError::InvalidRecord {
                line: line_num,
                detail: format!(
                    "expected at least {} columns, got {}",
                    max_idx + 1,
                    fields.len()
                ),
            });
        }

        let raw_ts = fields[ts_idx];
        let timestamp_ms = parse_timestamp(raw_ts).ok_or_else(|| CallsError::InvalidRecord {
            line: line_num,
            detail: format!("invalid timestamp: '{raw_ts}'"),
        })?;

        let caller = fields[caller_idx].to_string();
        let callee = fields[callee_idx].to_string();
        if caller.is_empty() || callee.is_empty() {
            return Err(CallsError::InvalidRecord {
                line: line_num,
                detail: "caller and callee must not be empty".into(),
            });
        }

        let duration_s: u32 = if let Some(idx) = duration_idx {
            if idx < fields.len() {
                fields[idx].parse().unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        let answered = if let Some(idx) = answered_idx {
            if idx < fields.len() {
                parse_bool_relaxed(fields[idx])
            } else {
                duration_s > 0
            }
        } else {
            duration_s > 0
        };

        let direction = if let Some(idx) = direction_idx {
            if idx < fields.len() {
                if fields[idx].eq_ignore_ascii_case("outgoing") {
                    CallDirection::Outgoing
                } else {
                    CallDirection::Incoming
                }
            } else {
                CallDirection::Incoming
            }
        } else {
            CallDirection::Incoming
        };

        let voicemail = if let Some(idx) = voicemail_idx {
            if idx < fields.len() {
                parse_bool_relaxed(fields[idx])
            } else {
                false
            }
        } else {
            false
        };

        records.push(CallRecord {
            timestamp_ms,
            caller,
            callee,
            duration_s,
            answered,
            direction,
            voicemail,
        });

        if records.len() > MAX_IMPORT_RECORDS {
            return Err(CallsError::TooManyRecords(
                records.len(),
                MAX_IMPORT_RECORDS,
            ));
        }
    }

    Ok(records)
}

fn parse_timestamp(s: &str) -> Option<i64> {
    if let Ok(num) = s.parse::<i64>() {
        // If in seconds (reasonable timestamp range 1e8..1e11), convert to ms
        return Some(if num < 100_000_000_000 {
            num * 1000
        } else {
            num
        });
    }
    // Try RFC 3339 / ISO 8601
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    None
}

fn parse_bool_relaxed(s: &str) -> bool {
    let lower = s.to_lowercase();
    matches!(
        lower.as_str(),
        "true" | "1" | "yes" | "y" | "answered" | "connected"
    )
}
