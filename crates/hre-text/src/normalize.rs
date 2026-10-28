//! Conservative Unicode, homoglyph, spacing, and character repetition normalizer.

use crate::error::TextError;

/// Maximum allowed text payload size in bytes (matches `AnalyzeRequest` bounds).
pub const MAX_TEXT_BYTES: usize = 32768;

/// Validates that text meets bounds constraints (non-empty and <= 32768 bytes).
pub fn validate_text(text: &str) -> Result<(), TextError> {
    if text.trim().is_empty() {
        return Err(TextError::Validation(
            "text payload must not be empty or whitespace only".into(),
        ));
    }
    if text.len() > MAX_TEXT_BYTES {
        return Err(TextError::Validation(format!(
            "text length ({} bytes) exceeds maximum allowed {} bytes",
            text.len(),
            MAX_TEXT_BYTES
        )));
    }
    Ok(())
}

/// Normalizes text conservatively:
/// 1. Strips zero-width and invisible control characters.
/// 2. Maps Cyrillic/Greek homoglyphs to ASCII equivalents.
/// 3. Normalizes fullwidth Latin characters and smart quotes.
/// 4. Collapses runs of 3+ identical consecutive characters to 2.
/// 5. Normalizes whitespace runs to a single space while trimming ends.
/// 6. Preserves quote structure (`"`, `'`, `>`) for quoted-abuse detection.
pub fn normalize(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Step 1: Character-level mapping (homoglyphs, invisible chars, quotes).
    let mut mapped = Vec::with_capacity(text.len());
    for ch in text.chars() {
        // Drop invisible / zero-width characters
        if matches!(
            ch,
            '\u{200B}' // Zero width space
                | '\u{200C}' // Zero width non-joiner
                | '\u{200D}' // Zero width joiner
                | '\u{200E}' // Left-to-right mark
                | '\u{200F}' // Right-to-left mark
                | '\u{FEFF}' // Zero width no-break space / BOM
                | '\u{00AD}' // Soft hyphen
                | '\u{2060}' // Word joiner
        ) {
            continue;
        }

        let mapped_ch = match ch {
            // Cyrillic lookalikes to Latin
            '\u{0430}' => 'a',
            '\u{0410}' => 'A',
            '\u{0441}' => 'c',
            '\u{0421}' => 'C',
            '\u{0435}' => 'e',
            '\u{0415}' => 'E',
            '\u{043E}' => 'o',
            '\u{041E}' => 'O',
            '\u{0440}' => 'p',
            '\u{0420}' => 'P',
            '\u{0445}' => 'x',
            '\u{0425}' => 'X',
            '\u{0443}' => 'y',
            '\u{0423}' => 'Y',
            '\u{0456}' => 'i',
            '\u{0406}' => 'I',
            '\u{0458}' => 'j',
            '\u{0408}' => 'J',
            '\u{0455}' => 's',
            '\u{0405}' => 'S',
            '\u{0501}' => 'd',
            '\u{0500}' => 'D',

            // Greek lookalikes to Latin
            '\u{0391}' => 'A',
            '\u{03B1}' => 'a',
            '\u{0392}' => 'B',
            '\u{03B2}' => 'b',
            '\u{0395}' => 'E',
            '\u{03B5}' => 'e',
            '\u{0397}' => 'H',
            '\u{03B7}' => 'n',
            '\u{0399}' => 'I',
            '\u{03B9}' => 'i',
            '\u{039A}' => 'K',
            '\u{03BA}' => 'k',
            '\u{039C}' => 'M',
            '\u{039D}' => 'N',
            '\u{03BD}' => 'v',
            '\u{039F}' => 'O',
            '\u{03BF}' => 'o',
            '\u{03A1}' => 'P',
            '\u{03C1}' => 'p',
            '\u{03A4}' => 'T',
            '\u{03C4}' => 't',
            '\u{03A7}' => 'X',
            '\u{03C7}' => 'x',
            '\u{03A5}' => 'Y',
            '\u{03C5}' => 'u',

            // Fullwidth Latin (0xFF01..=0xFF5E)
            c if ('\u{FF21}'..='\u{FF3A}').contains(&c) => {
                let code = (c as u32) - 0xFEE0;
                char::from_u32(code).unwrap_or(c)
            }
            c if ('\u{FF41}'..='\u{FF5A}').contains(&c) => {
                let code = (c as u32) - 0xFEE0;
                char::from_u32(code).unwrap_or(c)
            }

            // Smart / fancy quotes preserved as canonical quotes
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{00AB}' | '\u{00BB}' => '"',
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{02BB}' | '`' => '\'',

            // Whitespace variants
            '\u{00A0}' | '\u{2007}' | '\u{202F}' | '\u{3000}' | '\t' | '\r' | '\n' => ' ',

            other => other,
        };
        mapped.push(mapped_ch);
    }

    // Step 2: Collapse runs of 3+ consecutive identical characters to 2.
    // e.g. "sooooo" -> "soo", "killlll" -> "kill"
    let mut collapsed = Vec::with_capacity(mapped.len());
    let mut prev_char: Option<char> = None;
    let mut run_count = 0usize;

    for &c in &mapped {
        // Compare case-insensitively for run collapsing
        let is_same = prev_char.is_some_and(|p| p.to_lowercase().eq(c.to_lowercase()));
        if is_same {
            run_count += 1;
            if run_count <= 2 {
                collapsed.push(c);
            }
        } else {
            prev_char = Some(c);
            run_count = 1;
            collapsed.push(c);
        }
    }

    // Step 3: Whitespace consolidation (multiple spaces -> single space).
    let mut result = String::with_capacity(collapsed.len());
    let mut last_was_space = false;

    for c in collapsed {
        if c == ' ' {
            if !last_was_space && !result.is_empty() {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    // Trim trailing space if any
    if result.ends_with(' ') {
        result.pop();
    }

    result
}

/// Folds common leetspeak characters to ASCII Latin for cue/toxic lexicon matching.
pub fn fold_leetspeak(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let folded = match c {
            '@' => 'a',
            '$' => 's',
            '0' => 'o',
            '1' | '!' => 'i',
            '3' => 'e',
            '4' => 'a',
            '5' => 's',
            '7' => 't',
            '8' => 'b',
            other => other,
        };
        out.push(folded);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalization_unicode_and_spacing() {
        // Cyrillic 'е' (U+0435) and 'о' (U+043E) should normalize to Latin 'e' and 'o'
        let input = "H\u{0435}ll\u{043E}   world\u{200B}!";
        let normalized = normalize(input);
        assert_eq!(normalized, "Hello world!");

        // Repeated characters collapsed to 2
        let repeated = "soooooooo  coooollllll!";
        assert_eq!(normalize(repeated), "soo cooll!");

        // Smart quotes preserved
        let quotes = "She said: \u{201C}Leave me alone\u{201D}";
        assert_eq!(normalize(quotes), "She said: \"Leave me alone\"");

        // Fullwidth Latin
        let fullwidth = "\u{FF21}\u{FF4C}\u{FF49}\u{FF43}\u{FF45}";
        assert_eq!(normalize(fullwidth), "Alice");

        // Non-breaking space
        let nbsp = "hello\u{00A0}world";
        assert_eq!(normalize(nbsp), "hello world");
    }

    #[test]
    fn test_bounded_invalid_text() {
        // Empty
        assert!(validate_text("").is_err());
        assert!(validate_text("   \n\t  ").is_err());

        // Valid boundary
        assert!(validate_text("Hello").is_ok());

        // Oversized (> 32768 bytes)
        let huge = "a".repeat(MAX_TEXT_BYTES + 1);
        assert!(validate_text(&huge).is_err());

        let exactly_max = "a".repeat(MAX_TEXT_BYTES);
        assert!(validate_text(&exactly_max).is_ok());
    }

    #[test]
    fn test_leetspeak_folding() {
        let leet = "k1ll y0u";
        assert_eq!(fold_leetspeak(leet), "kill you");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn property_normalizer_never_panics(s in "\\PC*") {
            let out = normalize(&s);
            let _ = fold_leetspeak(&out);
        }
    }
}
