//! Indian-aware PII detection and masking.
//!
//! Detects:
//! - Indian phone numbers (+91 / 10 digits starting 6-9)
//! - Email addresses
//! - Aadhaar numbers (12 digits with Verhoeff checksum algorithm verification)
//! - PAN card numbers (5 letters, 4 digits, 1 letter)
//! - UPI IDs (user@bank)
//! - Indian PIN codes (6 digits, only when near address context cues)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiKind {
    Phone,
    Email,
    Aadhaar,
    Pan,
    Upi,
    Pin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiMatch {
    pub kind: PiiKind,
    pub masked_placeholder: String,
    pub start: usize,
    pub end: usize,
}

/// Dihedral group D5 multiplication table for Verhoeff algorithm.
const D_TABLE: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
    [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
    [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
    [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];

/// Permutation table for Verhoeff algorithm.
const P_TABLE: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// Inverse table for Verhoeff algorithm.
const INV_TABLE: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

/// Validates a sequence of digits using the Verhoeff checksum algorithm.
pub fn validate_verhoeff(digits: &[u8]) -> bool {
    if digits.is_empty() {
        return false;
    }
    let mut c = 0;
    for (i, &d) in digits.iter().rev().enumerate() {
        if d > 9 {
            return false;
        }
        let p = P_TABLE[i % 8][d as usize];
        c = D_TABLE[c as usize][p as usize];
    }
    c == 0
}

/// Generates the Verhoeff checksum digit for a number sequence.
pub fn generate_verhoeff(digits: &[u8]) -> u8 {
    let mut c = 0;
    for (i, &d) in digits.iter().rev().enumerate() {
        if d > 9 {
            return 0;
        }
        let p = P_TABLE[(i + 1) % 8][d as usize];
        c = D_TABLE[c as usize][p as usize];
    }
    INV_TABLE[c as usize]
}

/// Scans text and detects all Indian PII occurrences.
pub fn detect_pii(text: &str) -> Vec<PiiMatch> {
    let mut matches = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();

    // 1. Scan for emails
    let mut i = 0;
    while i < len {
        if bytes[i] == b'@' {
            // Find start of email
            let mut start = i;
            while start > 0 && is_email_user_char(bytes[start - 1]) {
                start -= 1;
            }
            // Find end of email
            let mut end = i + 1;
            let mut has_dot = false;
            while end < len && is_email_domain_char(bytes[end]) {
                if bytes[end] == b'.' {
                    has_dot = true;
                }
                end += 1;
            }
            while end > i + 1 && matches!(bytes[end - 1], b'.' | b'-') {
                end -= 1;
            }
            if start < i && end > i + 1 {
                let domain = &text[i + 1..end];
                if is_common_upi_handle(domain) {
                    matches.push(PiiMatch {
                        kind: PiiKind::Upi,
                        masked_placeholder: "[UPI]".into(),
                        start,
                        end,
                    });
                    i = end;
                    continue;
                } else if has_dot && end > i + 3 && !bytes[end - 1].is_ascii_punctuation() {
                    matches.push(PiiMatch {
                        kind: PiiKind::Email,
                        masked_placeholder: "[EMAIL]".into(),
                        start,
                        end,
                    });
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }

    // 2. Scan for PAN cards: [A-Z]{5}[0-9]{4}[A-Z]
    for w in 0..len.saturating_sub(9) {
        if (w == 0 || !bytes[w - 1].is_ascii_alphanumeric())
            && (w + 10 == len || !bytes[w + 10].is_ascii_alphanumeric())
        {
            let slice = &bytes[w..w + 10];
            let is_pan = slice[0..5].iter().all(u8::is_ascii_uppercase)
                && slice[5..9].iter().all(u8::is_ascii_digit)
                && slice[9].is_ascii_uppercase();
            if is_pan {
                matches.push(PiiMatch {
                    kind: PiiKind::Pan,
                    masked_placeholder: "[PAN]".into(),
                    start: w,
                    end: w + 10,
                });
            }
        }
    }

    // 3. Scan for Aadhaar (12 digits, optional spaces/hyphens in 4-4-4 format)
    let mut idx = 0;
    while idx < len {
        if bytes[idx].is_ascii_digit() && (idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric()) {
            let mut digit_buf = Vec::with_capacity(12);
            let mut end_idx = idx;
            while end_idx < len {
                let b = bytes[end_idx];
                if b.is_ascii_digit() {
                    digit_buf.push(b - b'0');
                } else if (b == b' ' || b == b'-') && (digit_buf.len() == 4 || digit_buf.len() == 8)
                {
                    // separator permitted between 4-digit blocks
                } else {
                    break;
                }
                end_idx += 1;
                if digit_buf.len() == 12 {
                    break;
                }
            }
            if digit_buf.len() == 12
                && (end_idx == len || !bytes[end_idx].is_ascii_alphanumeric())
                && validate_verhoeff(&digit_buf)
            {
                matches.push(PiiMatch {
                    kind: PiiKind::Aadhaar,
                    masked_placeholder: "[AADHAAR]".into(),
                    start: idx,
                    end: end_idx,
                });
                idx = end_idx;
                continue;
            }
        }
        idx += 1;
    }

    // 4. Scan for Indian Phone numbers:
    // +91 followed by 10 digits starting 6-9, or raw 10 digits starting 6-9
    let mut p_idx = 0;
    while p_idx < len {
        // Case A: +91 prefix
        if bytes[p_idx..].starts_with(b"+91") {
            let mut cur = p_idx + 3;
            while cur < len && (bytes[cur] == b' ' || bytes[cur] == b'-') {
                cur += 1;
            }
            if cur < len && (b'6'..=b'9').contains(&bytes[cur]) {
                let mut digits = 0;
                let mut end_cur = cur;
                while end_cur < len {
                    let b = bytes[end_cur];
                    if b.is_ascii_digit() {
                        digits += 1;
                    } else if b != b' ' && b != b'-' {
                        break;
                    }
                    end_cur += 1;
                    if digits == 10 {
                        break;
                    }
                }
                if digits == 10 && (end_cur == len || !bytes[end_cur].is_ascii_alphanumeric()) {
                    matches.push(PiiMatch {
                        kind: PiiKind::Phone,
                        masked_placeholder: "[PHONE]".into(),
                        start: p_idx,
                        end: end_cur,
                    });
                    p_idx = end_cur;
                    continue;
                }
            }
        }
        // Case B: 10 digits starting with 6-9 (word bounded)
        if (b'6'..=b'9').contains(&bytes[p_idx])
            && (p_idx == 0 || !bytes[p_idx - 1].is_ascii_alphanumeric())
        {
            let mut digits = 0;
            let mut end_cur = p_idx;
            while end_cur < len {
                let b = bytes[end_cur];
                if b.is_ascii_digit() {
                    digits += 1;
                } else if b != b' ' && b != b'-' {
                    break;
                }
                end_cur += 1;
                if digits == 10 {
                    break;
                }
            }
            if digits == 10 && (end_cur == len || !bytes[end_cur].is_ascii_alphanumeric()) {
                // Ensure this span wasn't already matched as Aadhaar
                let overlaps = matches.iter().any(|m| m.start <= p_idx && end_cur <= m.end);
                if !overlaps {
                    matches.push(PiiMatch {
                        kind: PiiKind::Phone,
                        masked_placeholder: "[PHONE]".into(),
                        start: p_idx,
                        end: end_cur,
                    });
                    p_idx = end_cur;
                    continue;
                }
            }
        }
        p_idx += 1;
    }

    // 5. Scan for Indian PIN codes: 6 digits starting with 1-9 near address cues
    let mut pin_idx = 0;
    while pin_idx + 6 <= len {
        if (b'1'..=b'9').contains(&bytes[pin_idx])
            && (pin_idx == 0 || !bytes[pin_idx - 1].is_ascii_alphanumeric())
            && (pin_idx + 6 == len || !bytes[pin_idx + 6].is_ascii_alphanumeric())
        {
            let slice = &bytes[pin_idx..pin_idx + 6];
            if slice.iter().all(u8::is_ascii_digit) {
                // Check if already matched
                let overlaps = matches
                    .iter()
                    .any(|m| pin_idx >= m.start && pin_idx + 6 <= m.end);
                if !overlaps {
                    // Check address context window (+/- 50 chars)
                    let mut ctx_start = pin_idx.saturating_sub(50);
                    let mut ctx_end = (pin_idx + 56).min(len);
                    while !text.is_char_boundary(ctx_start) {
                        ctx_start += 1;
                    }
                    while !text.is_char_boundary(ctx_end) {
                        ctx_end -= 1;
                    }
                    if has_address_cue(&text[ctx_start..ctx_end].to_lowercase()) {
                        matches.push(PiiMatch {
                            kind: PiiKind::Pin,
                            masked_placeholder: "[PIN]".into(),
                            start: pin_idx,
                            end: pin_idx + 6,
                        });
                        pin_idx += 6;
                        continue;
                    }
                }
            }
        }
        pin_idx += 1;
    }

    // Sort by start position and remove duplicates/overlaps
    matches.sort_by_key(|m| m.start);
    let mut deduped: Vec<PiiMatch> = Vec::new();
    for m in matches {
        if let Some(last) = deduped.last() {
            if m.start < last.end {
                continue; // Overlaps existing match
            }
        }
        deduped.push(m);
    }

    deduped
}

/// Masks detected PII in the input string and returns the masked text and list of matches.
pub fn mask_pii(text: &str) -> (String, Vec<PiiMatch>) {
    let matches = detect_pii(text);
    if matches.is_empty() {
        return (text.to_owned(), matches);
    }

    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in &matches {
        if m.start > last {
            out.push_str(&text[last..m.start]);
        }
        out.push_str(&m.masked_placeholder);
        last = m.end;
    }
    if last < text.len() {
        out.push_str(&text[last..]);
    }

    (out, matches)
}

fn is_email_user_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-' || b == b'+'
}

fn is_email_domain_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'.' || b == b'-'
}

fn is_common_upi_handle(domain: &str) -> bool {
    matches!(
        domain.to_ascii_lowercase().as_str(),
        "okhdfcbank"
            | "oksbi"
            | "okaxis"
            | "okicici"
            | "paytm"
            | "ybl"
            | "ibl"
            | "axl"
            | "apl"
            | "upi"
            | "barodampay"
            | "federal"
            | "kotak"
            | "indus"
    )
}

fn has_address_cue(ctx: &str) -> bool {
    const CUES: &[&str] = &[
        "address",
        "pin",
        "pincode",
        "postal",
        "near",
        "road",
        "street",
        "flat",
        "plot",
        "house",
        "sector",
        "nagar",
        "delhi",
        "mumbai",
        "bangalore",
        "bengaluru",
        "kolkata",
        "chennai",
        "hyderabad",
        "pune",
        "ahmedabad",
        "colony",
        "lane",
        "apartment",
    ];
    CUES.iter().any(|&cue| ctx.contains(cue))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_verhoeff_algorithm() {
        // 11 digits: 23661830889
        let base = [2, 3, 6, 6, 1, 8, 3, 0, 8, 8, 9];
        let check = generate_verhoeff(&base);
        let mut full = base.to_vec();
        full.push(check);
        assert!(validate_verhoeff(&full));

        // Corrupted digit
        full[5] = (full[5] + 1) % 10;
        assert!(!validate_verhoeff(&full));
    }

    #[test]
    fn test_aadhaar_requires_verhoeff() {
        let base = [2, 3, 6, 6, 1, 8, 3, 0, 8, 8, 9];
        let check = generate_verhoeff(&base);
        let valid_aadhaar = format!("23661830889{check}");
        let invalid_aadhaar = "236618308890"; // Invalid checksum

        let matches_valid = detect_pii(&format!("User aadhaar is {valid_aadhaar}"));
        assert!(matches_valid.iter().any(|m| m.kind == PiiKind::Aadhaar));

        let matches_invalid = detect_pii(&format!("User number is {invalid_aadhaar}"));
        assert!(!matches_invalid.iter().any(|m| m.kind == PiiKind::Aadhaar));
    }

    #[test]
    fn test_indian_phone_and_email_masked() {
        let raw = "Call +91 9876543210 or email test.user@example.com immediately";
        let (masked, matches) = mask_pii(raw);
        assert_eq!(masked, "Call [PHONE] or email [EMAIL] immediately");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_pan_card_masked() {
        let raw = "My PAN card is ABCDE1234F for verification";
        let (masked, matches) = mask_pii(raw);
        assert_eq!(masked, "My PAN card is [PAN] for verification");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, PiiKind::Pan);
    }

    #[test]
    fn test_upi_masked() {
        let raw = "Send money to merchant@okhdfcbank right now";
        let (masked, matches) = mask_pii(raw);
        assert_eq!(masked, "Send money to [UPI] right now");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, PiiKind::Upi);
    }

    #[test]
    fn test_pin_needs_address_context() {
        // Without address context, 6 digits should NOT be flagged as PIN
        let no_ctx = "Your confirmation code is 560001 today";
        let matches = detect_pii(no_ctx);
        assert!(!matches.iter().any(|m| m.kind == PiiKind::Pin));

        // With address context, 6 digits SHOULD be flagged as PIN
        let with_ctx = "Delivery address near MG Road pincode 560001";
        let (masked, matches) = mask_pii(with_ctx);
        assert!(matches.iter().any(|m| m.kind == PiiKind::Pin));
        assert!(masked.contains("[PIN]"));
    }
}
