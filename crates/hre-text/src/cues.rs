//! Threat, coercion, extortion, and location cue detection.

/// Detected cue categories.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CueMatches {
    pub threat_cues: Vec<String>,
    pub coercion_cues: Vec<String>,
    pub location_cues: Vec<String>,
    pub is_quoted: bool,
}

const THREAT_CUES: &[&str] = &[
    "i will kill you",
    "i will hurt you",
    "i will find you",
    "die bitch",
    "slit your",
    "beat you up",
    "end your life",
    "watch your back",
    "destroy you",
    "you will regret",
    "cut you",
    "strangle you",
];

const COERCION_CUES: &[&str] = &[
    "pay me",
    "send money",
    "send cash",
    "transfer money",
    "bitcoin",
    "leak your photos",
    "leak your pictures",
    "expose your photos",
    "expose your pictures",
    "send nudes",
    "send naked",
    "or else i will post",
    "share with your family",
    "share with your friends",
];

const LOCATION_CUES: &[&str] = &[
    "outside your",
    "i saw you at",
    "i know where you live",
    "following you",
    "watching you",
    "behind you",
    "standing outside",
    "at your door",
    "outside your window",
    "in front of your house",
    "at your office",
];

/// Scans normalized lowercased text for behavioral and threat cues.
pub fn detect_cues(normalized_lower: &str) -> CueMatches {
    let mut matches = CueMatches {
        is_quoted: check_is_quoted(normalized_lower),
        ..Default::default()
    };

    for &cue in THREAT_CUES {
        if normalized_lower.contains(cue) {
            matches.threat_cues.push(cue.into());
        }
    }

    for &cue in COERCION_CUES {
        if normalized_lower.contains(cue) {
            matches.coercion_cues.push(cue.into());
        }
    }

    for &cue in LOCATION_CUES {
        if normalized_lower.contains(cue) {
            matches.location_cues.push(cue.into());
        }
    }

    matches
}

/// Checks if the text represents quoted speech or reported abuse rather than direct hostility.
fn check_is_quoted(text: &str) -> bool {
    let trimmed = text.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('“') && trimmed.ends_with('”'))
        || trimmed.starts_with('>')
    {
        return true;
    }

    // Common reporting prefixes
    const REPORTING_PREFIXES: &[&str] = &[
        "he said \"",
        "she said \"",
        "they said \"",
        "he told me \"",
        "she told me \"",
        "message said \"",
        "quoted: \"",
    ];

    let lower = text.to_lowercase();
    REPORTING_PREFIXES.iter().any(|&p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cue_detection() {
        let text = "i will kill you if you don't pay me and leak your photos outside your house";
        let cues = detect_cues(text);
        assert!(!cues.threat_cues.is_empty());
        assert!(!cues.coercion_cues.is_empty());
        assert!(!cues.location_cues.is_empty());
        assert!(!cues.is_quoted);
    }

    #[test]
    fn test_quoted_abuse_detection() {
        let quoted = "> i will kill you";
        let cues = detect_cues(quoted);
        assert!(cues.is_quoted);

        let reporting = "He said \"i will kill you\" when he called";
        let cues2 = detect_cues(reporting);
        assert!(cues2.is_quoted);
    }
}
