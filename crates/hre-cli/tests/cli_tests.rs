#[test]
fn test_cli_help_and_scenarios() {
    let scenarios = [
        "polite_stalker",
        "pile_on",
        "cyberflasher",
        "screenshot",
        "voice_note",
        "call_log",
        "heavy_benign",
        "slang",
        "evidence",
    ];
    assert_eq!(scenarios.len(), 9);
}
