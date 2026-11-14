use super::*;

#[test]
fn analysis_accepts_only_its_identity_and_observational_option() {
    assert_eq!(
        parsed("analyze memo --dry-run --json").unwrap().verb,
        Verb::Analyze {
            id: "memo".into(),
            dry_run: true,
            resend_refused: false
        }
    );
    assert_eq!(
        parsed("analyze memo --resend-refused").unwrap().verb,
        Verb::Analyze {
            id: "memo".into(),
            dry_run: false,
            resend_refused: true
        }
    );
    for input in [
        "analyze --all --resend-refused",
        "analyze --stdin --resend-refused",
        "analyze",
        "analyze one two",
        "analyze memo --language en",
        "analyze memo --dry-run --dry-run",
        "analyze memo --correct private",
    ] {
        assert!(parsed(input).is_err(), "{input}");
    }
}

#[test]
fn bulk_analysis_selects_all_or_stdin_and_refuses_mixed_forms() {
    assert_eq!(
        parsed("analyze --all --since 2026-08-24T14:47:00-06:00 --dry-run")
            .unwrap()
            .verb,
        Verb::AnalyzeBatch {
            scope: crate::cli::args::analyze::Scope::All {
                since: Some(vpt_domain::time::UtcInstant {
                    secs: 1_787_604_420
                })
            },
            dry_run: true
        }
    );
    assert_eq!(
        parsed("analyze --stdin").unwrap().verb,
        Verb::AnalyzeBatch {
            scope: crate::cli::args::analyze::Scope::Stdin,
            dry_run: false
        }
    );
    for input in [
        "analyze memo --all",
        "analyze --all memo",
        "analyze --stdin memo",
        "analyze --all --stdin",
        "analyze --stdin --since 2026-08-24T14:47:00Z",
        "analyze memo --since 2026-08-24T14:47:00Z",
        "analyze --all --since yesterday",
        "analyze --all --since",
        "analyze --all --all",
    ] {
        assert!(parsed(input).is_err(), "{input}");
    }
}
