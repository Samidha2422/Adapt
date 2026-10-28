//! Indian-aware Personally Identifiable Information (PII) detection and masking.
//!
//! Handles:
//! - Email addresses
//! - Indian phone numbers (+91 / 10-digit starting with 6, 7, 8, 9)
//! - Aadhaar numbers (12 digits verified with the Verhoeff checksum algorithm)
//! - PAN (Permanent Account Number: 5 uppercase letters, 4 digits, 1 uppercase letter)
//! - UPI Virtual Payment Addresses (e.g., username@bank / username@upi)
//! - Indian Postal PIN codes (6 digits, ONLY flagged when adjacent to address cues)

use serde::{Deserialize, Serialize};

/// Dihedral group D5 multiplication table for the Verhoeff checksum algorithm.
const VERHOEFF_D: [[u8; 10]; 10] = [
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

/// Permutation table for the Verhoeff checksum algorithm.
const VERHOEFF_P: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// Multiplicative inverse table in D5 for computing Verhoeff check digits.
pub const VERHOEFF_INV: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

/// Validates a sequence of decimal digits using the Verhoeff algorithm.
pub fn validate_verhoeff(digits: &[u8]) -> bool {
    if digits.is_empty() {
        return false;
    }
    let mut c = 0u8;
    for (i, &d) in digits.iter().rev().enumerate() {
        if d > 9 {
            return false;
        }
        let p_val = VERHOEFF_P[i % 8][d as usize] as usize;
        c = VERHOEFF_D[c as usize][p_val];
    }
    c == 0
}

/// Computes the Verhoeff check digit for a given sequence of digits.
pub fn compute_verhoeff_check_digit(digits: &[u8]) -> u8 {
    let mut c = 0u8;
    for (i, &d) in digits.iter().rev().enumerate() {
        if d <= 9 {
            let p_val = VERHOEFF_P[(i + 1) % 8][d as usize] as usize;
            c = VERHOEFF_D[c as usize][p_val];
        }
    }
    VERHOEFF_INV[c as usize]
}

/// Category of detected PII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiKind {
    Email,
    Phone,
    Aadhaar,
    Pan,
    Upi,
    PinCode,
}

/// Single detected PII instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiiItem {
    pub kind: PiiKind,
    pub masked: String,
    pub start: usize,
    pub end: usize,
}

/// Aggregate PII detection output with masked content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PiiDetectionResult {
    pub items: Vec<PiiItem>,
    pub masked_text: String,
    pub contains_address: bool,
    pub has_pii: bool,
    pub score: f64,
}

/// Address context cues used to qualify 6-digit postal PIN codes.
const ADDRESS_CUES: &[&str] = &[
    "address",
    "addr",
    "house",
    "flat",
    "street",
    "road",
    "rd",
    "lane",
    "cross",
    "sector",
    "sec",
    "colony",
    "nagar",
    "city",
    "pincode",
    "pin code",
    "pin:",
    "pin -",
    "delhi",
    "mumbai",
    "bengaluru",
    "bangalore",
    "hyderabad",
    "chennai",
    "kolkata",
    "pune",
    "ahmedabad",
    "jaipur",
    "apartment",
    "apt",
    "building",
    "bldg",
    "near",
    "opposite",
    "opp",
    "behind",
];

/// Known UPI handle suffixes.
const UPI_HANDLES: &[&str] = &[
    "oksbi",
    "okhdfcbank",
    "okaxis",
    "okicici",
    "paytm",
    "ybl",
    "ibl",
    "axl",
    "upi",
    "apl",
    "slice",
    "postpe",
    "jupiteraxis",
    "idfcbank",
    "freecharge",
    "kotak",
    "barodampay",
    "indus",
    "fbl",
    "abfspay",
    "airtel",
    "yesbank",
];

/// Scans text for Indian-aware PII and returns masked text and detected items.
pub fn detect_and_mask_pii(text: &str) -> PiiDetectionResult {
    let mut items = Vec::new();
    let lower_text = text.to_lowercase();

    // 1. Detect Email Addresses: [user]@[domain].[tld]
    detect_emails(text, &mut items);

    // 2. Detect UPI VPAs: [user]@[handle]
    detect_upis(text, &mut items);

    // 3. Detect Aadhaar (12 digits with Verhoeff validation)
    detect_aadhaar(text, &mut items);

    // 4. Detect PAN (5 uppercase letters, 4 digits, 1 uppercase letter)
    detect_pan(text, &mut items);

    // 5. Detect Indian Phone numbers (+91 / 10-digit starting with 6..9)
    detect_phones(text, &mut items);

    // 6. Detect PIN codes (6-digit starting with 1..9 near address cues)
    let contains_address = check_address_context(&lower_text);
    if contains_address {
        detect_pin_codes(text, &lower_text, &mut items);
    }

    // Sort items by start position ascending
    items.sort_by_key(|item| item.start);

    // Remove overlapping spans (keep the earlier / longer match)
    let mut non_overlapping: Vec<PiiItem> = Vec::with_capacity(items.len());
    for item in items {
        if let Some(prev) = non_overlapping.last() {
            if item.start < prev.end {
                continue; // Skip overlap
            }
        }
        non_overlapping.push(item);
    }

    // Build masked text by replacing spans in reverse order
    let mut masked_text = text.to_string();
    for item in non_overlapping.iter().rev() {
        if item.start < masked_text.len() && item.end <= masked_text.len() {
            masked_text.replace_range(item.start..item.end, &item.masked);
        }
    }

    let has_pii = !non_overlapping.is_empty();
    let score = if !has_pii {
        0.0
    } else {
        let mut max_score: f64 = 0.4;
        for it in &non_overlapping {
            match it.kind {
                PiiKind::Aadhaar | PiiKind::Pan => max_score = max_score.max(0.7_f64),
                PiiKind::Phone | PiiKind::Email => max_score = max_score.max(0.5_f64),
                PiiKind::PinCode if contains_address => max_score = max_score.max(0.6_f64),
                _ => {}
            }
        }
        max_score
    };

    PiiDetectionResult {
        items: non_overlapping,
        masked_text,
        contains_address,
        has_pii,
        score,
    }
}

fn check_address_context(lower_text: &str) -> bool {
    ADDRESS_CUES
        .iter()
        .any(|&cue| has_word_match(lower_text, cue))
}

fn has_word_match(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(pos) = text[start..].find(word) {
        let abs_pos = start + pos;
        let end_pos = abs_pos + word.len();
        let before_ok = abs_pos == 0 || !bytes[abs_pos - 1].is_ascii_alphanumeric();
        let after_ok = end_pos == bytes.len() || !bytes[end_pos].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = end_pos;
    }
    false
}

fn is_word_boundary(bytes: &[u8], idx: usize) -> bool {
    if idx == 0 || idx >= bytes.len() {
        return true;
    }
    let prev = bytes[idx - 1];
    let curr = bytes[idx];
    !(prev.is_ascii_alphanumeric() && curr.is_ascii_alphanumeric())
}

/// Detects email addresses like user@example.com
fn detect_emails(text: &str, items: &mut Vec<PiiItem>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'@' {
            // Find local part start
            let mut local_start = i;
            while local_start > 0 {
                let c = bytes[local_start - 1];
                if c.is_ascii_alphanumeric()
                    || c == b'.'
                    || c == b'_'
                    || c == b'%'
                    || c == b'+'
                    || c == b'-'
                {
                    local_start -= 1;
                } else {
                    break;
                }
            }

            // Find domain part end
            let mut domain_end = i + 1;
            let mut dot_seen = false;
            while domain_end < len {
                let c = bytes[domain_end];
                if c == b'.' {
                    dot_seen = true;
                    domain_end += 1;
                } else if c.is_ascii_alphanumeric() || c == b'-' {
                    domain_end += 1;
                } else {
                    break;
                }
            }

            // Verify valid email structure
            if local_start < i && domain_end > i + 1 && dot_seen {
                let domain = &text[i + 1..domain_end];
                if let Some(tld_idx) = domain.rfind('.') {
                    let tld = &domain[tld_idx + 1..];
                    if tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()) {
                        let local_str = &text[local_start..i];
                        let masked_local = if local_str.len() <= 2 {
                            format!("{}***", &local_str[..1])
                        } else {
                            format!(
                                "{}***{}",
                                &local_str[..1],
                                &local_str[local_str.len() - 1..]
                            )
                        };
                        let masked = format!("{masked_local}@{domain}");
                        items.push(PiiItem {
                            kind: PiiKind::Email,
                            masked,
                            start: local_start,
                            end: domain_end,
                        });
                        i = domain_end;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
}

/// Detects UPI Virtual Payment Addresses (e.g. user@oksbi)
fn detect_upis(text: &str, items: &mut Vec<PiiItem>) {
    let lower = text.to_lowercase();
    for &handle in UPI_HANDLES {
        let pattern = format!("@{handle}");
        let mut start_idx = 0;
        while let Some(pos) = lower[start_idx..].find(&pattern) {
            let at_idx = start_idx + pos;
            let vpa_end = at_idx + pattern.len();

            // Check word boundary after handle
            let boundary_after =
                vpa_end == text.len() || !text.as_bytes()[vpa_end].is_ascii_alphanumeric();

            if boundary_after {
                // Find VPA username start
                let mut vpa_start = at_idx;
                while vpa_start > 0 {
                    let b = text.as_bytes()[vpa_start - 1];
                    if b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_' {
                        vpa_start -= 1;
                    } else {
                        break;
                    }
                }

                if at_idx > vpa_start {
                    let user_part = &text[vpa_start..at_idx];
                    let masked_user = if user_part.len() <= 2 {
                        format!("{}***", &user_part[..1])
                    } else {
                        format!(
                            "{}***{}",
                            &user_part[..1],
                            &user_part[user_part.len() - 1..]
                        )
                    };
                    let masked = format!("{masked_user}{pattern}");
                    items.push(PiiItem {
                        kind: PiiKind::Upi,
                        masked,
                        start: vpa_start,
                        end: vpa_end,
                    });
                }
            }
            start_idx = vpa_end;
        }
    }
}

/// Detects 12-digit Aadhaar numbers and verifies them with the Verhoeff algorithm.
fn detect_aadhaar(text: &str, items: &mut Vec<PiiItem>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Aadhaar first digit is 2..=9
        if bytes[i] >= b'2' && bytes[i] <= b'9' && (i == 0 || !bytes[i - 1].is_ascii_digit()) {
            let mut digits = Vec::with_capacity(12);
            let mut j = i;

            while j < len && digits.len() < 12 {
                let b = bytes[j];
                if b.is_ascii_digit() {
                    digits.push(b - b'0');
                    j += 1;
                } else if (b == b' ' || b == b'-') && !digits.is_empty() && digits.len() % 4 == 0 {
                    j += 1; // separator between 4-digit groups (e.g. 1234 5678 9012)
                } else {
                    break;
                }
            }

            if digits.len() == 12 {
                let boundary_after = j == len || !bytes[j].is_ascii_digit();
                if boundary_after && validate_verhoeff(&digits) {
                    let last4_str =
                        format!("{}{}{}{}", digits[8], digits[9], digits[10], digits[11]);
                    let masked = format!("XXXX-XXXX-{last4_str}");
                    items.push(PiiItem {
                        kind: PiiKind::Aadhaar,
                        masked,
                        start: i,
                        end: j,
                    });
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Detects Indian PAN cards: 5 uppercase letters, 4 digits, 1 uppercase letter.
fn detect_pan(text: &str, items: &mut Vec<PiiItem>) {
    let bytes = text.as_bytes();
    let len = bytes.len();

    if len < 10 {
        return;
    }

    for i in 0..=len - 10 {
        if !is_word_boundary(bytes, i) || !is_word_boundary(bytes, i + 10) {
            continue;
        }

        let slice = &bytes[i..i + 10];
        // 5 uppercase letters
        let first5_letters = slice[..5].iter().all(|b| b.is_ascii_uppercase());
        // 4 digits
        let next4_digits = slice[5..9].iter().all(|b| b.is_ascii_digit());
        // 1 uppercase letter
        let last_letter = slice[9].is_ascii_uppercase();

        if first5_letters && next4_digits && last_letter {
            let last4 = std::str::from_utf8(&slice[5..9]).unwrap_or("XXXX");
            let masked = format!("XXXXX{last4}X");
            items.push(PiiItem {
                kind: PiiKind::Pan,
                masked,
                start: i,
                end: i + 10,
            });
        }
    }
}

/// Detects Indian phone numbers (+91 / 10-digit starting with 6, 7, 8, 9).
fn detect_phones(text: &str, items: &mut Vec<PiiItem>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Check for +91 / 91 / 0 prefix or standalone 10-digit starting with 6..9
        let mut start = i;
        let mut has_country_code = false;

        if bytes[i..].starts_with(b"+91") && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            has_country_code = true;
            start = i;
            i += 3;
            // Skip space or hyphen after country code
            if i < len && (bytes[i] == b' ' || bytes[i] == b'-') {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"91") && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
        {
            if i + 2 < len && (bytes[i + 2] == b' ' || bytes[i + 2] == b'-') {
                has_country_code = true;
                start = i;
                i += 3;
            }
        } else if bytes[i] == b'0'
            && (i == 0 || !bytes[i - 1].is_ascii_digit())
            && i + 1 < len
            && bytes[i + 1] >= b'6'
            && bytes[i + 1] <= b'9'
        {
            start = i;
            i += 1;
        }

        // Must start with 6, 7, 8, or 9
        if i < len
            && bytes[i] >= b'6'
            && bytes[i] <= b'9'
            && (start == i || has_country_code || (i > 0 && !bytes[i - 1].is_ascii_digit()))
        {
            let phone_start = start;
            let mut digits = Vec::with_capacity(10);
            let mut j = i;

            while j < len && digits.len() < 10 {
                let b = bytes[j];
                if b.is_ascii_digit() {
                    digits.push(b);
                    j += 1;
                } else if (b == b' ' || b == b'-') && digits.len() < 10 {
                    j += 1;
                } else {
                    break;
                }
            }

            if digits.len() == 10 {
                let boundary_after = j == len || !bytes[j].is_ascii_digit();
                if boundary_after {
                    let last4 = std::str::from_utf8(&digits[6..10]).unwrap_or("XXXX");
                    let masked = if has_country_code {
                        format!("+91-XXXXX-{last4}")
                    } else {
                        format!("XXXXX-{last4}")
                    };
                    items.push(PiiItem {
                        kind: PiiKind::Phone,
                        masked,
                        start: phone_start,
                        end: j,
                    });
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Detects 6-digit Indian PIN codes ONLY when near address cues.
fn detect_pin_codes(text: &str, lower_text: &str, items: &mut Vec<PiiItem>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // PIN code starts with 1..=9, exactly 6 digits
        if bytes[i] >= b'1' && bytes[i] <= b'9' && (i == 0 || !bytes[i - 1].is_ascii_digit()) {
            let mut digits_count = 0;
            let mut j = i;

            while j < len && bytes[j].is_ascii_digit() {
                digits_count += 1;
                j += 1;
            }

            if digits_count == 6 {
                let boundary_after = j == len || !bytes[j].is_ascii_digit();
                if boundary_after {
                    // Check local window (+/- 50 characters) for address cues
                    let window_start = i.saturating_sub(50);
                    let window_end = (j + 50).min(len);
                    let local_window = &lower_text[window_start..window_end];

                    let has_local_cue = ADDRESS_CUES
                        .iter()
                        .any(|&cue| has_word_match(local_window, cue));
                    if has_local_cue {
                        items.push(PiiItem {
                            kind: PiiKind::PinCode,
                            masked: "[PIN_REDACTED]".to_string(),
                            start: i,
                            end: j,
                        });
                        i = j;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indian_phone_and_email_masked() {
        let input = "Contact me at +91-9876543210 or email test.user@example.com for info";
        let res = detect_and_mask_pii(input);

        assert!(res.has_pii);
        assert!(res.items.iter().any(|it| it.kind == PiiKind::Phone));
        assert!(res.items.iter().any(|it| it.kind == PiiKind::Email));

        assert!(res.masked_text.contains("+91-XXXXX-3210"));
        assert!(res.masked_text.contains("t***r@example.com"));
        assert!(!res.masked_text.contains("9876543210"));
        assert!(!res.masked_text.contains("test.user@example.com"));

        // Standalone 10-digit phone
        let input2 = "Call 9876543210 immediately";
        let res2 = detect_and_mask_pii(input2);
        assert!(res2.has_pii);
        assert!(res2.masked_text.contains("XXXXX-3210"));
    }

    #[test]
    fn test_aadhaar_requires_verhoeff() {
        // Generate valid Aadhaar using Verhoeff
        let base_11 = [2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 0u8, 1u8, 2u8];
        let check = compute_verhoeff_check_digit(&base_11);
        let valid_aadhaar = format!("2345 6789 012{}", check);

        let input_valid = format!("My Aadhaar is {valid_aadhaar}");
        let res_valid = detect_and_mask_pii(&input_valid);
        assert!(
            res_valid.items.iter().any(|it| it.kind == PiiKind::Aadhaar),
            "Valid Verhoeff Aadhaar must be detected"
        );
        assert!(res_valid.masked_text.contains("XXXX-XXXX-"));

        // Corrupted checksum (change last digit)
        let bad_check = (check + 1) % 10;
        let invalid_aadhaar = format!("2345 6789 012{bad_check}");
        let input_invalid = format!("My Aadhaar is {invalid_aadhaar}");
        let res_invalid = detect_and_mask_pii(&input_invalid);
        assert!(
            !res_invalid
                .items
                .iter()
                .any(|it| it.kind == PiiKind::Aadhaar),
            "Invalid Verhoeff Aadhaar must NOT be flagged as Aadhaar"
        );
    }

    #[test]
    fn test_pin_needs_address_context() {
        // Without address context: 6-digit number should NOT be flagged as PIN
        let order_text = "Your tracking order ID is 560001 please track online";
        let order_res = detect_and_mask_pii(order_text);
        assert!(
            !order_res.items.iter().any(|it| it.kind == PiiKind::PinCode),
            "Isolated 6-digit number without address cues must not be flagged as PIN code"
        );
        assert_eq!(order_res.masked_text, order_text);

        // With address context: Should be detected and masked
        let addr_text = "Send it to flat 402, brigade road, bengaluru 560001 please";
        let addr_res = detect_and_mask_pii(addr_text);
        assert!(
            addr_res.items.iter().any(|it| it.kind == PiiKind::PinCode),
            "6-digit number near address cues must be detected as PIN code"
        );
        assert!(addr_res.masked_text.contains("[PIN_REDACTED]"));
        assert!(!addr_res.masked_text.contains("560001"));
    }

    #[test]
    fn test_pan_card_detection() {
        let text = "My PAN card number is ABCDE1234F for tax purposes";
        let res = detect_and_mask_pii(text);
        assert!(res.items.iter().any(|it| it.kind == PiiKind::Pan));
        assert!(res.masked_text.contains("XXXXX1234X"));
        assert!(!res.masked_text.contains("ABCDE1234F"));
    }

    #[test]
    fn test_upi_detection() {
        let text = "Pay me via my UPI id john.doe@oksbi right now";
        let res = detect_and_mask_pii(text);
        assert!(res.items.iter().any(|it| it.kind == PiiKind::Upi));
        assert!(res.masked_text.contains("@oksbi"));
        assert!(!res.masked_text.contains("john.doe@oksbi"));
    }
}
