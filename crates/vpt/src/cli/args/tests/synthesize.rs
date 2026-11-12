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

#[test]
fn bulk_synthesis_selects_all_or_stdin_and_refuses_mixed_forms() {
    assert_eq!(
        parsed("synthesize --all --since 2026-08-24T14:47:00-06:00 --dry-run")
            .unwrap()
            .verb,
        Verb::SynthesizeBatch {
            scope: crate::cli::args::synthesize::Scope::All {
                since: Some(vpt_domain::time::UtcInstant {
                    secs: 1_787_604_420
                })
            },
            dry_run: true
        }
    );
    assert_eq!(
        parsed("synthesize --stdin").unwrap().verb,
        Verb::SynthesizeBatch {
            scope: crate::cli::args::synthesize::Scope::Stdin,
            dry_run: false
        }
    );
    for input in [
        "synthesize memo --all",
        "synthesize --all memo",
        "synthesize --stdin memo",
        "synthesize --all --stdin",
        "synthesize --stdin --since 2026-08-24T14:47:00Z",
        "synthesize memo --since 2026-08-24T14:47:00Z",
        "synthesize --all --since yesterday",
        "synthesize --all --since",
        "synthesize --all --all",
    ] {
        assert!(parsed(input).is_err(), "{input}");
    }
}
