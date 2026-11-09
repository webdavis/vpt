use super::*;

#[test]
fn synthesis_accepts_only_its_identity_and_observational_option() {
    assert_eq!(
        parsed("synthesize memo --dry-run --json").unwrap().verb,
        Verb::Synthesize {
            id: "memo".into(),
            dry_run: true
        }
    );
    for input in [
        "synthesize",
        "synthesize one two",
        "synthesize memo --language en",
        "synthesize memo --dry-run --dry-run",
        "synthesize memo --correct private",
    ] {
        assert!(parsed(input).is_err(), "{input}");
    }
}
