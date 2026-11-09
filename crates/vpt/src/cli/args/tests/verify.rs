use super::*;

#[test]
fn verification_requires_a_path_and_recording_and_accepts_global_options() {
    assert_eq!(
        parsed("verify-note note.md --recording memo --json")
            .unwrap()
            .verb,
        Verb::VerifyNote {
            path: "note.md".into(),
            recording: "memo".into()
        }
    );
    for args in [
        "verify-note",
        "verify-note note.md",
        "verify-note --recording memo",
        "verify-note note.md --recording",
        "verify-note note.md --recording memo --recording other",
        "verify-note note.md --recording memo --dry-run",
    ] {
        assert!(parsed(args).is_err(), "{args}");
    }
}
