//! Threat, coercion, location, sextortion pattern, and quoted abuse cue detection.

use serde::{Deserialize, Serialize};

/// Threat cue phrases and their severity weight.
const SEVERE_THREAT_CUES: &[&str] = &[
    "i will kill you",
    "i'll kill you",
    "going to kill you",
    "gonna kill you",
    "i will murder you",
    "i will hurt you",
    "i'll hurt you",
    "i will end you",
    "i will break your",
    "i will beat you",
    "i will slit your",
    "i will shoot you",
    "i will stab you",
    "i will hunt you down",
    "you will die",
    "go die",
    "hope you die",
    "die bitch",
    "watch your back",
    // Common Hinglish threat phrases
    "mar dunga",
    "maar daalunga",
    "jaan se maar dunga",
    "dekh lunga tujhe",
];

/// Coercion and blackmail cues.
const COERCION_CUES: &[&str] = &[
    "or else",
    "do what i say",
    "you better listen",
    "you have no choice",
    "if you don't",
    "i will ruin you",
    "i will ruin your life",
    "i will expose you",
    "you will regret this",
    "i'm warning you",
    "do as i tell you",
];

/// Leak, intimate media, and payment demand cues (sextortion components).
const LEAK_PAYMENT_CUES: &[&str] = &[
    "leak your photos",
    "leak your pictures",
    "leak your videos",
    "post your pictures",
    "upload your photos",
    "send nudes",
    "send nude",
    "send intimate",
    "expose your pictures",
    "pay me",
    "transfer money",
    "send money",
    "bitcoin",
    "crypto",
    "rupees",
    "cash",
];

/// Physical location, surveillance, and stalking cues.
const LOCATION_CUES: &[&str] = &[
    "i saw you at",
    "i saw you today",
    "i know where you live",
    "i know where you work",
    "outside your house",
    "outside your door",
    "standing outside",
    "waiting outside",
    "waiting for you at",
    "watching you",
    "following you",
    "near your house",
    "near your building",
    "i know your address",
];

/// Results of cue extraction and contextual analysis.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CueMatches {
    /// Inferred threat score from cues (0.0 to 1.0).
    pub threat_score: f64,
    /// Explicit threat phrases detected.
    pub threat_cues: Vec<String>,
    /// Coercion phrases detected.
    pub coercion_cues: Vec<String>,
    /// Leak / payment demand phrases detected.
    pub leak_payment_cues: Vec<String>,
    /// Physical location / stalking phrases detected.
    pub location_cues: Vec<String>,
    /// Whether sextortion pattern conditions are met (simulated signal).
    pub sextortion_pattern: bool,
    /// Whether abuse appears predominantly inside quotes (victim/reporter context).
    pub quoted_abuse: bool,
    /// Fraction of abusive cues located inside quotes (0.0 to 1.0).
    pub quoted_fraction: f64,
    /// Indicates simulation provenance (e.g. sextortion pattern).
    pub simulated: bool,
}

/// Identifies byte ranges of quoted text (e.g. `"..."`, `'...'`, or `> ...`).
pub fn find_quoted_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Double quotes
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'"' {
                i += 1;
            }
            if i < len && bytes[i] == b'"' {
                spans.push((start, i + 1));
                i += 1;
                continue;
            }
        }
        // Single quotes (at least 3 characters inside to distinguish from contractions like don't)
        else if bytes[i] == b'\''
            && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b':' || bytes[i - 1] == b'\n')
        {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'\'' {
                i += 1;
            }
            if i < len && bytes[i] == b'\'' && (i - start > 3) {
                spans.push((start, i + 1));
                i += 1;
                continue;
            }
        }
        // Markdown blockquote line (`> ...`)
        else if bytes[i] == b'>' && (i == 0 || bytes[i - 1] == b'\n') {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            spans.push((start, i));
            continue;
        }
        i += 1;
    }

    spans
}

fn is_in_spans(pos: usize, spans: &[(usize, usize)]) -> bool {
    spans.iter().any(|&(start, end)| pos >= start && pos < end)
}

/// Analyzes normalized text for threat, coercion, location, sextortion, and quoted abuse cues.
pub fn analyze_cues(text: &str) -> CueMatches {
    let lower = text.to_lowercase();
    let quoted_spans = find_quoted_spans(text);

    let mut threat_cues = Vec::new();
    let mut coercion_cues = Vec::new();
    let mut leak_payment_cues = Vec::new();
    let mut location_cues = Vec::new();

    let mut total_toxic_cues = 0usize;
    let mut quoted_toxic_cues = 0usize;

    // 1. Scan Threat Cues
    for &cue in SEVERE_THREAT_CUES {
        let mut start_idx = 0;
        while let Some(pos) = lower[start_idx..].find(cue) {
            let abs_pos = start_idx + pos;
            if !threat_cues.contains(&cue.to_string()) {
                threat_cues.push(cue.to_string());
            }
            total_toxic_cues += 1;
            if is_in_spans(abs_pos, &quoted_spans) {
                quoted_toxic_cues += 1;
            }
            start_idx = abs_pos + cue.len();
        }
    }

    // 2. Scan Coercion Cues
    for &cue in COERCION_CUES {
        let mut start_idx = 0;
        while let Some(pos) = lower[start_idx..].find(cue) {
            let abs_pos = start_idx + pos;
            if !coercion_cues.contains(&cue.to_string()) {
                coercion_cues.push(cue.to_string());
            }
            total_toxic_cues += 1;
            if is_in_spans(abs_pos, &quoted_spans) {
                quoted_toxic_cues += 1;
            }
            start_idx = abs_pos + cue.len();
        }
    }

    // 3. Scan Leak / Payment Cues
    for &cue in LEAK_PAYMENT_CUES {
        let mut start_idx = 0;
        while let Some(pos) = lower[start_idx..].find(cue) {
            let abs_pos = start_idx + pos;
            if !leak_payment_cues.contains(&cue.to_string()) {
                leak_payment_cues.push(cue.to_string());
            }
            total_toxic_cues += 1;
            if is_in_spans(abs_pos, &quoted_spans) {
                quoted_toxic_cues += 1;
            }
            start_idx = abs_pos + cue.len();
        }
    }

    // 4. Scan Location Cues
    for &cue in LOCATION_CUES {
        let mut start_idx = 0;
        while let Some(pos) = lower[start_idx..].find(cue) {
            let abs_pos = start_idx + pos;
            if !location_cues.contains(&cue.to_string()) {
                location_cues.push(cue.to_string());
            }
            start_idx = abs_pos + cue.len();
        }
    }

    // Compute quoted abuse fraction
    let (quoted_abuse, quoted_fraction) = if total_toxic_cues > 0 {
        let frac = (quoted_toxic_cues as f64) / (total_toxic_cues as f64);
        (frac >= 0.75, frac)
    } else {
        (false, 0.0)
    };

    // Calculate threat score
    let base_threat_score = if threat_cues.is_empty() {
        0.0
    } else if threat_cues.iter().any(|c| {
        c.contains("kill")
            || c.contains("murder")
            || c.contains("die")
            || c.contains("mar dunga")
            || c.contains("slit")
    }) {
        0.95
    } else {
        0.75
    };

    // Quoted abuse reduces active threat score
    let threat_score = if quoted_abuse {
        (base_threat_score * 0.2_f64).min(0.3_f64)
    } else {
        base_threat_score
    };

    // Sextortion pattern: Coercion + (Leak OR Payment) + (Threat OR intimate request)
    // Always flagged as simulated signal, not a definitive verdict
    let has_coercion = !coercion_cues.is_empty();
    let has_leak_or_pay = !leak_payment_cues.is_empty();
    let sextortion_pattern = !quoted_abuse && has_coercion && has_leak_or_pay;
    let simulated = sextortion_pattern;

    CueMatches {
        threat_score,
        threat_cues,
        coercion_cues,
        leak_payment_cues,
        location_cues,
        sextortion_pattern,
        quoted_abuse,
        quoted_fraction,
        simulated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_and_location_cues() {
        let text = "I will kill you. I know where you live and I saw you at the station.";
        let cues = analyze_cues(text);

        assert!(cues.threat_score >= 0.9);
        assert!(cues.threat_cues.contains(&"i will kill you".to_string()));
        assert!(cues
            .location_cues
            .contains(&"i know where you live".to_string()));
        assert!(cues.location_cues.contains(&"i saw you at".to_string()));
        assert!(!cues.quoted_abuse);
    }

    #[test]
    fn test_quoted_abuse_not_auto_verdict() {
        // Victim reporting what someone told them
        let victim_report = "The stalker texted me \"I will kill you\" and I am terrified";
        let cues = analyze_cues(victim_report);

        assert!(
            cues.quoted_abuse,
            "Abusive phrase enclosed in quotes with reporting context must be flagged as quoted abuse"
        );
        assert!(
            cues.threat_score < 0.5,
            "Quoted abuse must discount threat score so victim is not auto-verdicted"
        );
        assert!(cues.quoted_fraction >= 0.8);
    }

    #[test]
    fn test_sextortion_pattern_simulation() {
        let text = "Do what I say or else I will leak your photos! Pay me in bitcoin now.";
        let cues = analyze_cues(text);

        assert!(
            cues.sextortion_pattern,
            "Coercion + leak photos + payment cues must trigger sextortion pattern"
        );
        assert!(
            cues.simulated,
            "Sextortion pattern must set simulated = true"
        );
    }
}
