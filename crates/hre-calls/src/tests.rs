use super::*;
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn pseudonymizer() -> Result<Pseudonymizer, hre_core::CoreError> {
    Pseudonymizer::new(b"test-only-012345678901234567890123")
}

fn csv(rows: &str) -> Vec<u8> {
    format!("{}\n{rows}", CSV_COLUMNS.join(",")).into_bytes()
}

fn row(id: u128, time: &str, caller: &str, callee: &str, flags: &str) -> String {
    format!("{},{time},{caller},{callee},{flags}\n", Uuid::from_u128(id))
}

#[test]
fn import_handles_bom_crlf_quotes_reordered_headers_and_pseudonymizes() -> TestResult {
    let p = pseudonymizer()?;
    let input = concat!(
        "\u{feff}simulated,event_id,occurred_at,caller_id,callee_id,duration_s,answered,voicemail,after_refusal\r\n",
        "true,00000000-0000-0000-0000-000000000001,2026-10-28T23:30:00+05:30,",
        "\"caller,\"\"quoted\"\"\",callee,4,true,false,true\r\n"
    );
    let records = import_csv(input.as_bytes(), &p, "calls")?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].caller, p.subject("calls", "caller,\"quoted\"")?);
    assert_eq!(
        records[0].occurred_at.to_rfc3339(),
        "2026-10-28T18:00:00+00:00"
    );
    assert!(records[0].after_refusal && records[0].simulated);
    let output = serde_json::to_string(&records)?;
    assert!(!output.contains("quoted"));
    assert!(!format!("{records:?}").contains("quoted"));
    assert_ne!(records[0].caller, p.subject("other", "caller,\"quoted\"")?);
    Ok(())
}

#[test]
fn import_rejects_malformed_rows_atomically_without_exposing_values() -> TestResult {
    let p = pseudonymizer()?;
    let good = row(
        1,
        "2026-10-28T18:00:00Z",
        "private-caller",
        "target",
        "0,false,false,false,false",
    );
    for bad in [
        "not-a-uuid,2026-10-28T18:00:00Z,private-caller,target,0,false,false,false,false\n",
        "00000000-0000-0000-0000-000000000002,bad-time,private-caller,target,0,false,false,false,false\n",
        "00000000-0000-0000-0000-000000000002,2026-10-28T18:00:00Z,private-caller,target,-1,false,false,false,false\n",
        "00000000-0000-0000-0000-000000000002,2026-10-28T18:00:00Z,private-caller,target,1,FALSE,false,false,false\n",
        "\"unterminated\n",
        "a\"b,c\n",
        "\"a\"x,c\n",
        "a,b,\n",
        "\n",
    ] {
        let result = import_csv(&csv(&(good.clone() + bad)), &p, "calls");
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(!error.to_string().contains("private-caller"));
            assert!(!format!("{error:?}").contains("private-caller"));
        }
    }
    Ok(())
}

#[test]
fn import_rejects_duplicate_ids_unknown_duplicate_headers_and_invalid_utf8() -> TestResult {
    let p = pseudonymizer()?;
    let one = row(
        1,
        "2026-10-28T18:00:00Z",
        "a",
        "b",
        "0,false,false,false,false",
    );
    assert!(import_csv(&csv(&(one.clone() + &one)), &p, "calls").is_err());
    for header in [
        CSV_COLUMNS.join(",").replace("answered", "unknown"),
        CSV_COLUMNS.join(",").replace("answered", "simulated"),
    ] {
        assert!(import_csv(header.as_bytes(), &p, "calls").is_err());
    }
    assert!(import_csv(&[0xff], &p, "calls").is_err());
    assert!(import_csv(&[], &p, "calls").is_err());
    assert!(import_csv(&csv(""), &p, "").is_err());
    assert!(import_csv(&csv(""), &p, "calls")?.is_empty());
    Ok(())
}

#[test]
fn import_enforces_identity_duration_and_timestamp_boundaries() -> TestResult {
    let p = pseudonymizer()?;
    for (caller, time, flags) in [
        ("", "2026-10-28T18:00:00Z", "0,false,false,false,false"),
        (" a", "2026-10-28T18:00:00Z", "0,false,false,false,false"),
        ("a\tb", "2026-10-28T18:00:00Z", "0,false,false,false,false"),
        ("b", "2026-10-28T18:00:00Z", "0,false,false,false,false"),
        ("a", "2101-01-01T00:00:00Z", "0,false,false,false,false"),
        ("a", "2026-10-28T18:00:00Z", "86401,false,false,false,false"),
        (
            "a",
            "2026-10-28T18:00:00Z",
            "4294967296,false,false,false,false",
        ),
        ("a", "2026-10-28T18:00:00Z", "+1,false,false,false,false"),
    ] {
        assert!(import_csv(&csv(&row(1, time, caller, "b", flags)), &p, "calls").is_err());
    }
    assert!(import_csv(
        &csv(&row(
            1,
            "2026-10-28T18:00:00Z",
            &"a".repeat(256),
            "b",
            "86400,true,false,false,false"
        )),
        &p,
        "calls"
    )
    .is_ok());
    assert!(import_csv(
        &csv(&row(
            1,
            "2026-10-28T18:00:00Z",
            &"a".repeat(257),
            "b",
            "0,false,false,false,false"
        )),
        &p,
        "calls"
    )
    .is_err());
    Ok(())
}

#[test]
fn import_enforces_byte_line_and_record_limits() -> TestResult {
    let p = pseudonymizer()?;
    assert!(matches!(
        import_csv(&vec![b'a'; MAX_CSV_BYTES + 1], &p, "calls"),
        Err(CallError::Limit("csv_bytes"))
    ));
    assert!(matches!(
        import_csv(&vec![b'a'; 4097], &p, "calls"),
        Err(CallError::Limit("line_bytes"))
    ));
    let mut rows = String::new();
    for i in 1..=MAX_RECORDS {
        rows.push_str(&row(
            i as u128,
            "2026-10-28T18:00:00Z",
            "a",
            "b",
            "0,false,false,false,false",
        ));
    }
    assert_eq!(import_csv(&csv(&rows), &p, "calls")?.len(), MAX_RECORDS);
    rows.push_str(&row(
        10001,
        "2026-10-28T18:00:00Z",
        "a",
        "b",
        "0,false,false,false,false",
    ));
    assert!(matches!(
        import_csv(&csv(&rows), &p, "calls"),
        Err(CallError::Limit("records"))
    ));
    Ok(())
}

#[test]
fn golden_vector_includes_boundaries_excludes_future_and_tracks_reverse_and_target() -> TestResult {
    let p = pseudonymizer()?;
    let mut rows = String::new();
    // Unsorted, one-hour boundary, day boundary, seven-day boundary, future and unrelated.
    for (id, time, caller, callee, flags) in [
        (
            1,
            "2026-10-28T18:00:00Z",
            "a",
            "b",
            "0,false,false,true,true",
        ),
        (
            2,
            "2026-10-28T17:00:00Z",
            "a",
            "b",
            "4,true,true,false,false",
        ),
        (
            3,
            "2026-10-27T18:00:00Z",
            "a",
            "b",
            "600,true,false,false,false",
        ),
        (
            4,
            "2026-10-21T18:00:00Z",
            "a",
            "b",
            "0,false,false,false,false",
        ),
        (
            5,
            "2026-10-28T18:00:01Z",
            "a",
            "b",
            "0,false,false,true,true",
        ),
        (
            6,
            "2026-10-28T17:30:00Z",
            "b",
            "a",
            "30,true,false,false,false",
        ),
        (
            7,
            "2026-10-28T17:30:00Z",
            "c",
            "b",
            "0,false,false,false,false",
        ),
        (
            8,
            "2026-10-28T17:30:00Z",
            "x",
            "y",
            "0,false,false,false,false",
        ),
        (
            9,
            "2026-10-21T17:59:59Z",
            "a",
            "b",
            "0,false,false,false,false",
        ),
    ] {
        rows.push_str(&row(id, time, caller, callee, flags));
    }
    let calls = import_csv(&csv(&rows), &p, "calls")?;
    let now = DateTime::parse_from_rfc3339("2026-10-28T18:00:00Z")?.with_timezone(&Utc);
    let offset = FixedOffset::east_opt(19_800).ok_or("offset")?;
    let features = extract_features(
        &calls,
        &p.subject("calls", "a")?,
        &p.subject("calls", "b")?,
        now,
        offset,
    )?;
    assert_eq!(
        features.model_vector(),
        [2., 3., 1., 3., 1., 604., 2., 0.25]
    );
    assert_eq!(features.seven_days.call_count, 4);
    assert_eq!(features.one_day.short_answered_calls, 1);
    assert_eq!(features.one_day.voicemail_count, 1);
    assert_eq!(features.max_calls_1h, 2);
    assert!(features.gap_regularity.is_some());
    assert!(features.simulated);
    assert!(matches!(features.status, Status::Simulated));
    assert_eq!(features.new_number_flag, None);
    Ok(())
}

#[test]
fn empty_history_has_no_inferred_new_number_or_reciprocity() -> TestResult {
    let p = pseudonymizer()?;
    let now = DateTime::parse_from_rfc3339("2026-10-28T18:00:00Z")?.with_timezone(&Utc);
    let offset = FixedOffset::east_opt(0).ok_or("offset")?;
    let features = extract_features(
        &[],
        &p.subject("calls", "a")?,
        &p.subject("calls", "b")?,
        now,
        offset,
    )?;
    assert_eq!(features.model_vector(), [0.; 8]);
    assert!(!features.simulated);
    assert!(matches!(features.status, Status::Built));
    assert_eq!(features.gap_regularity, None);
    assert_eq!(features.max_calls_1h, 0);
    Ok(())
}

#[test]
fn extraction_revalidates_public_records_and_rejects_duplicates() -> TestResult {
    let p = pseudonymizer()?;
    let mut calls = import_csv(
        &csv(&row(
            1,
            "2026-10-28T18:00:00Z",
            "a",
            "b",
            "0,false,false,false,false",
        )),
        &p,
        "calls",
    )?;
    let caller = p.subject("calls", "a")?;
    let callee = p.subject("calls", "b")?;
    let offset = FixedOffset::east_opt(0).ok_or("offset")?;
    let now = calls[0].occurred_at;
    calls.push(calls[0].clone());
    assert!(extract_features(&calls, &caller, &callee, now, offset).is_err());
    calls.pop();
    calls[0].caller = SubjectId("raw-sensitive-id".into());
    assert!(extract_features(&calls, &caller, &callee, now, offset).is_err());
    Ok(())
}

#[test]
fn night_boundaries_use_recipient_time_and_short_calls_do_not_imply_silence() -> TestResult {
    let p = pseudonymizer()?;
    let rows = row(
        1,
        "2026-10-28T16:29:59Z",
        "a",
        "b",
        "5,true,false,false,false",
    ) + &row(
        2,
        "2026-10-28T16:30:00Z",
        "a",
        "b",
        "4,true,false,false,false",
    ) + &row(
        3,
        "2026-10-28T23:29:59Z",
        "a",
        "b",
        "100,true,false,false,false",
    ) + &row(
        4,
        "2026-10-28T23:30:00Z",
        "a",
        "b",
        "0,false,false,false,false",
    );
    let calls = import_csv(&csv(&rows), &p, "calls")?;
    let offset = FixedOffset::east_opt(19_800).ok_or("offset")?;
    let features = extract_features(
        &calls,
        &calls[0].caller,
        &calls[0].callee,
        calls[3].occurred_at,
        offset,
    )?;
    assert_eq!(features.one_day.night_calls, 2);
    assert_eq!(features.one_day.short_answered_calls, 1);
    Ok(())
}

#[test]
fn simulation_from_reverse_or_target_counts_propagates_but_future_does_not() -> TestResult {
    let p = pseudonymizer()?;
    let base = row(
        1,
        "2026-10-28T18:00:00Z",
        "a",
        "b",
        "600,true,false,false,false",
    );
    for (caller, callee, time, expected) in [
        ("b", "a", "2026-10-28T18:00:00Z", true),
        ("c", "b", "2026-10-28T18:00:00Z", true),
        ("x", "y", "2026-10-28T18:00:00Z", false),
        ("a", "b", "2026-10-28T18:00:01Z", false),
    ] {
        let calls = import_csv(
            &csv(&(base.clone() + &row(2, time, caller, callee, "0,false,false,false,true"))),
            &p,
            "calls",
        )?;
        let features = extract_features(
            &calls,
            &calls[0].caller,
            &calls[0].callee,
            calls[0].occurred_at,
            FixedOffset::east_opt(0).ok_or("offset")?,
        )?;
        assert_eq!(features.simulated, expected);
    }
    Ok(())
}

#[test]
fn distinct_simultaneous_calls_count_without_inventing_gap_regularity() -> TestResult {
    let p = pseudonymizer()?;
    let mut rows = String::new();
    for id in 1..=3 {
        rows.push_str(&row(
            id,
            "2026-10-28T12:00:00Z",
            "a",
            "b",
            "600,true,false,false,false",
        ));
    }
    rows.push_str(&row(
        4,
        "2026-10-28T12:00:00Z",
        "b",
        "a",
        "600,true,false,false,false",
    ));
    let calls = import_csv(&csv(&rows), &p, "calls")?;
    let features = extract_features(
        &calls,
        &calls[0].caller,
        &calls[0].callee,
        calls[0].occurred_at,
        FixedOffset::east_opt(0).ok_or("offset")?,
    )?;
    assert_eq!(features.max_calls_1h, 3);
    assert_eq!(features.gap_regularity, None);
    assert_eq!(features.one_day.unanswered_ratio, 0.0);
    assert_eq!(features.one_day.avg_duration_s, 600.0);
    assert_eq!(features.one_day.after_refusal_calls, 0);
    assert_eq!(features.recipient_reply_ratio_24h, 0.25);
    Ok(())
}

proptest! {
    #[test]
    fn arbitrary_csv_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..8192)) {
        if let Ok(p) = pseudonymizer() {
            let _ = import_csv(&bytes, &p, "calls");
        }
    }
}
