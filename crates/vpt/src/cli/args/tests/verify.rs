use super::*;

#[test]
fn verification_requires_a_path_and_recording_and_accepts_global_options() {
    assert_eq!(
        parsed("note verify note.md --recording memo -o json")
            .unwrap()
            .verb,
        Verb::VerifyNote {
            path: "note.md".into(),
            recording: "memo".into()
        }
    );
    for args in [
        "note verify",
        "note verify note.md",
        "note verify --recording memo",
        "note verify note.md --recording",
        "note verify note.md --recording memo --recording other",
        "note verify note.md --recording memo --preview",
        "verify-note note.md --recording memo",
    ] {
        assert!(parsed(args).is_err(), "{args}");
    }
}

#[test]
fn an_occasion_path_is_its_own_subcommand() {
    assert_eq!(
        parsed("path occasion meeting --stage brief -o json")
            .unwrap()
            .verb,
        Verb::OccasionPath {
            id: "meeting".into(),
            stage: "brief".into()
        }
    );
    for line in [
        "path --occasion meeting --stage brief",
        "path occasion --stage brief",
        "path occasion meeting",
        "path memo occasion meeting --stage brief",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}
