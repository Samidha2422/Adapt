//! Integration tests for the hre-text pipeline covering all Card A3 requirements.

use hre_core::{Backend, Status};
use hre_text::{
    analyze, analyze_detailed, compute_verhoeff_check_digit, detect_and_mask_pii, normalize,
    validate_text, HeuristicTextBackend, PiiKind, TextBackend, TextError, MAX_TEXT_BYTES,
};

#[test]
fn test_normalization_unicode_and_spacing() {
    // 1. Unicode zero-width spaces stripped
    let zwsp_input = "k\u{200B}i\u{200B}l\u{200B}l";
    assert_eq!(normalize(zwsp_input), "kill");

    // 2. Cyrillic homoglyph lookalikes converted to Latin ASCII
    // U+0430 is Cyrillic small 'a', U+0435 is Cyrillic small 'e'
    let homoglyph_input = "H\u{0435}llo b\u{0430}by";
    assert_eq!(normalize(homoglyph_input), "Hello baby");

    // 3. Repeated characters collapsed to maximum 2
    let repeated = "whyyyyyyy sooooo   angryyyyyy";
    assert_eq!(normalize(repeated), "whyy soo angryy");

    // 4. Whitespace variants (tabs, newlines, non-breaking spaces) collapsed to single spaces
    let spacing = "first\t\tline\n\n\nsecond\u{00A0}line";
    assert_eq!(normalize(spacing), "first line second line");

    // 5. Quote structure preserved for downstream quoted-abuse detection
    let quotes = "She said: \u{201C}leave me alone\u{201D} and \u{2018}stop\u{2019}";
    assert_eq!(normalize(quotes), "She said: \"leave me alone\" and 'stop'");
}

#[test]
fn test_missing_model_degraded() -> Result<(), TextError> {
    let backend = HeuristicTextBackend::new();
    let res = backend.analyze_detailed("Just a normal friendly message.")?;

    // Must report Built status with Heuristic backend
    assert_eq!(res.signal.status, Status::Built);
    assert_eq!(res.signal.backend, Backend::Heuristic);

    // Must report text.onnx_missing in degradation list
    assert!(
        res.degraded.contains(&"text.onnx_missing".to_string()),
        "Missing model must report text.onnx_missing"
    );

    // Score must be low for benign text
    assert!(res.signal.score < 0.2);
    assert_eq!(res.signal.label, "benign");
    Ok(())
}

#[test]
fn test_indian_phone_and_email_masked() {
    let text = "Reach out to me at +91 98765 43210 or user.name@domain.co.in right away!";
    let res = detect_and_mask_pii(text);

    assert!(res.has_pii);

    // Assert phone is masked
    assert!(
        res.masked_text.contains("+91-XXXXX-3210"),
        "Masked text must contain masked phone: {}",
        res.masked_text
    );
    assert!(!res.masked_text.contains("98765 43210"));

    // Assert email is masked
    assert!(
        res.masked_text.contains("u***e@domain.co.in"),
        "Masked text must contain masked email: {}",
        res.masked_text
    );
    assert!(!res.masked_text.contains("user.name@domain.co.in"));

    // Standalone 10-digit phone
    let standalone = "My alternate is 8765432109 call now";
    let standalone_res = detect_and_mask_pii(standalone);
    assert!(standalone_res.masked_text.contains("XXXXX-2109"));
}

#[test]
fn test_aadhaar_requires_verhoeff() {
    // Construct a valid 12-digit Aadhaar using the Verhoeff check digit
    let base = [3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 0u8, 1u8, 2u8, 3u8];
    let check_digit = compute_verhoeff_check_digit(&base);
    let valid_aadhaar = format!("3456 7890 123{check_digit}");

    let valid_input = format!("Target Aadhaar: {valid_aadhaar}");
    let valid_res = detect_and_mask_pii(&valid_input);

    assert!(
        valid_res.items.iter().any(|it| it.kind == PiiKind::Aadhaar),
        "Aadhaar with valid Verhoeff checksum MUST be detected"
    );
    assert!(valid_res.masked_text.contains("XXXX-XXXX-"));

    // Invalidate checksum
    let bad_check = (check_digit + 1) % 10;
    let invalid_aadhaar = format!("3456 7890 123{bad_check}");
    let invalid_input = format!("Target Aadhaar: {invalid_aadhaar}");
    let invalid_res = detect_and_mask_pii(&invalid_input);

    assert!(
        !invalid_res
            .items
            .iter()
            .any(|it| it.kind == PiiKind::Aadhaar),
        "12 digits failing Verhoeff checksum MUST NOT be flagged as Aadhaar"
    );
}

#[test]
fn test_pin_needs_address_context() {
    // Standalone 6 digits without address cues -> NOT a PIN
    let isolated = "Your confirmation code is 560034 to login.";
    let isolated_res = detect_and_mask_pii(isolated);
    assert!(
        !isolated_res
            .items
            .iter()
            .any(|it| it.kind == PiiKind::PinCode),
        "6-digit number without address cues must not be flagged as a postal PIN code"
    );
    assert!(!isolated_res.masked_text.contains("[PIN_REDACTED]"));

    // 6 digits near address cues -> FLAGGED and masked
    let with_address = "I know your house near koramangala, bengaluru 560034";
    let addr_res = detect_and_mask_pii(with_address);
    assert!(
        addr_res.items.iter().any(|it| it.kind == PiiKind::PinCode),
        "6-digit number near address cues must be detected as PIN code"
    );
    assert!(addr_res.masked_text.contains("[PIN_REDACTED]"));
    assert!(!addr_res.masked_text.contains("560034"));
}

#[test]
fn test_quoted_abuse_not_auto_verdict() -> Result<(), TextError> {
    // When text quotes abuse in victim or report context
    let victim_message =
        "He kept sending me messages like: \"I will kill you and break your face\" so I blocked him";
    let res = analyze_detailed(victim_message)?;

    assert!(
        res.cues.quoted_abuse,
        "Abusive cues inside quotation marks must be recognized as quoted abuse"
    );
    assert!(
        res.cues.quoted_fraction >= 0.75,
        "Quoted fraction must be >= 75%"
    );

    // Quoted abuse must NOT result in a high threat score or high severity verdict
    assert!(
        res.cues.threat_score <= 0.35,
        "Threat score must be discounted for quoted abuse"
    );
    assert!(
        res.signal.score <= 0.35,
        "Signal toxicity must be discounted for quoted abuse"
    );
    Ok(())
}

#[test]
fn test_bounded_invalid_text() {
    // Empty string
    assert!(matches!(validate_text(""), Err(TextError::Validation(_))));

    // Whitespace only
    assert!(matches!(
        validate_text("   \n\t   "),
        Err(TextError::Validation(_))
    ));

    // Exceeds 32768 bytes
    let oversize = "x".repeat(MAX_TEXT_BYTES + 1);
    assert!(matches!(
        validate_text(&oversize),
        Err(TextError::Validation(_))
    ));

    // Valid text within bounds
    assert!(validate_text("A normal valid message within bounds").is_ok());
}

#[test]
fn test_end_to_end_analyze_threat() -> Result<(), TextError> {
    let severe_threat = "I will kill you if you ever talk to them again!";
    let signal = analyze(severe_threat)?;

    assert_eq!(signal.label, "threat");
    assert!(signal.score >= 0.9);
    assert_eq!(signal.status, Status::Built);
    assert_eq!(signal.backend, Backend::Heuristic);
    Ok(())
}
