use super::*;
use crate::cli::args::analyze::Scope;

#[test]
fn analysis_accepts_only_its_identity_and_observational_option() {
    assert_eq!(
        parsed("analyze memo --preview --json").unwrap().verb,
        Verb::Analyze {
            id: "memo".into(),
            dry_run: true,
        }
    );
    for input in [
        "analyze memo --resend-refused",
        "analyze",
        "analyze one two",
        "analyze memo --language en",
        "analyze memo --preview --preview",
        "analyze memo --correct private",
    ] {
        assert!(parsed(input).is_err(), "{input}");
    }
}

#[test]
fn bulk_analysis_is_the_all_subcommand_or_ids_on_stdin() {
    assert_eq!(
        parsed("analyze all --since 2026-08-24T14:47:00-06:00 --preview")
            .unwrap()
            .verb,
        Verb::AnalyzeBatch {
            scope: Scope::All {
                since: Some(vpt_domain::time::UtcInstant {
                    secs: 1_787_604_420
                })
            },
            dry_run: true
        }
    );
    for line in ["analyze -", "analyze - --json"] {
        assert_eq!(
            parsed(line).unwrap().verb,
            Verb::AnalyzeBatch {
                scope: Scope::Stdin,
                dry_run: false
            }
        );
    }
    assert_eq!(
        parsed("analyze - -n").unwrap().verb,
        Verb::AnalyzeBatch {
            scope: Scope::Stdin,
            dry_run: true
        }
    );
    for input in [
        "analyze --all",
        "analyze --stdin",
        "analyze memo all",
        "analyze all memo",
        "analyze - memo",
        "analyze - --since 2026-08-24T14:47:00Z",
        "analyze memo --since 2026-08-24T14:47:00Z",
        "analyze all --since yesterday",
        "analyze all --since",
        "analyze all --resend-refused",
    ] {
        assert!(parsed(input).is_err(), "{input}");
    }
}
