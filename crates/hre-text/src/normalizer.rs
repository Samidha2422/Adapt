//! Text normalization: Unicode homoglyphs, repeated characters, spacing, and leetspeak.

/// Maximum allowed text length in bytes.
pub const MAX_TEXT_LENGTH_BYTES: usize = 32_768;

/// Normalizes text by removing zero-width characters, normalizing homoglyphs,
/// compressing repeated characters, and collapsing whitespace.
pub fn normalize(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(input.len());
    let mut prev_char: Option<char> = None;
    let mut repeat_count = 0;
    let mut prev_is_space = false;

    for ch in input.chars() {
        // Skip zero-width / invisible formatting characters
        if is_zero_width(ch) {
            continue;
        }

        // Map homoglyphs to canonical ASCII / standard form
        let canon = map_homoglyph(ch);

        // Normalize whitespace to standard ASCII space
        if canon.is_whitespace() {
            if !prev_is_space && !out.is_empty() {
                out.push(' ');
                prev_is_space = true;
            }
            prev_char = None;
            repeat_count = 0;
            continue;
        }
        prev_is_space = false;

        // Compress consecutive identical letters > 2 to at most 2 (leave digits unchanged)
        let lower = canon.to_ascii_lowercase();
        if canon.is_alphabetic() && Some(lower) == prev_char {
            repeat_count += 1;
            if repeat_count < 2 {
                out.push(canon);
            }
        } else {
            prev_char = if canon.is_alphabetic() {
                Some(lower)
            } else {
                None
            };
            repeat_count = 0;
            out.push(canon);
        }
    }

    // Trim trailing whitespace
    if out.ends_with(' ') {
        out.pop();
    }

    out
}

/// Normalizes leetspeak replacements for keyword / cue matching.
pub fn normalize_leetspeak(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let mapped = match ch {
            '@' => 'a',
            '$' => 's',
            '0' => 'o',
            '1' | '!' | '|' => 'i',
            '3' => 'e',
            '4' => 'a',
            '5' => 's',
            '7' => 't',
            '8' => 'b',
            other => other,
        };
        out.push(mapped);
    }
    out
}

fn is_zero_width(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' // Zero-width space
            | '\u{200C}' // Zero-width non-joiner
            | '\u{200D}' // Zero-width joiner
            | '\u{200E}' // Left-to-right mark
            | '\u{200F}' // Right-to-left mark
            | '\u{FEFF}' // Zero-width no-break space (BOM)
            | '\u{00AD}' // Soft hyphen
            | '\u{2060}' // Word joiner
    )
}

fn map_homoglyph(c: char) -> char {
    match c {
        // Cyrillic small letters matching Latin
        'а' => 'a',
        'в' => 'b',
        'е' => 'e',
        'к' => 'k',
        'м' => 'm',
        'н' => 'h',
        'о' => 'o',
        'р' => 'p',
        'с' => 'c',
        'т' => 't',
        'у' => 'y',
        'х' => 'x',
        'і' => 'i',
        'ј' => 'j',
        'ѕ' => 's',

        // Cyrillic capital letters matching Latin
        'А' => 'A',
        'В' => 'B',
        'Е' => 'E',
        'К' => 'K',
        'М' => 'M',
        'Н' => 'H',
        'О' => 'O',
        'Р' => 'P',
        'С' => 'C',
        'Т' => 'T',
        'У' => 'Y',
        'Х' => 'X',
        'І' => 'I',
        'Ј' => 'J',
        'Ѕ' => 'S',

        // Greek small letters matching Latin
        'α' => 'a',
        'β' => 'b',
        'ε' => 'e',
        'ι' => 'i',
        'κ' => 'k',
        'ν' => 'v',
        'ο' => 'o',
        'ρ' => 'p',
        'τ' => 't',
        'υ' => 'u',
        'χ' => 'x',

        // Greek capital letters matching Latin
        'Α' => 'A',
        'Β' => 'B',
        'Ε' => 'E',
        'Η' => 'H',
        'Ι' => 'I',
        'Κ' => 'K',
        'Μ' => 'M',
        'Ν' => 'N',
        'Ο' => 'O',
        'Ρ' => 'P',
        'Τ' => 'T',
        'Υ' => 'Y',
        'Χ' => 'X',
        'Ζ' => 'Z',

        // Fullwidth ASCII (U+FF01 to U+FF5E)
        '\u{FF01}'..='\u{FF5E}' => {
            let code = c as u32;
            char::from_u32(code - 0xFEE0).unwrap_or(c)
        }

        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalization_unicode_and_spacing() {
        let raw = "  Hеllo    wоrld! \u{200B}\u{FEFF}  ";
        // 'е' and 'о' are Cyrillic homoglyphs
        let norm = normalize(raw);
        assert_eq!(norm, "Hello world!");
    }

    #[test]
    fn test_repeated_characters_compressed() {
        let raw = "soooooo goooood byeeee";
        assert_eq!(normalize(raw), "soo good byee");
    }

    #[test]
    fn test_leetspeak() {
        let raw = "k!ll y0u";
        assert_eq!(normalize_leetspeak(raw), "kill you");
    }
}
