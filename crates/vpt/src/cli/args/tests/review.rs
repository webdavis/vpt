use super::*;
use vpt_domain::review::ReviewAction;

#[test]
fn review_resolution_keeps_literal_corrections_separate_from_options_and_debug() {
    for text in [
        "Private canary",
        "--json",
        "--term",
        "--correct",
        "--config",
        "--resolve",
        "--confirm",
    ] {
        let invocation = parse(
            [
                "vpt",
                "--json",
                "review",
                "memo",
                "--correct",
                text,
                "--resolve",
                "flag",
                "--config",
                "settings.toml",
            ]
            .map(OsString::from),
        )
        .unwrap();
        assert_eq!(invocation.format, Some(Format::Json));
        assert_eq!(invocation.config, Some("settings.toml".into()));
        assert!(!format!("{invocation:?}").contains(text));
        assert_eq!(
            invocation.verb,
            Verb::ReviewResolve {
                id: "memo".into(),
                flag: "flag".into(),
                action: ReviewAction::Correct(text.into())
            }
        );
    }
    assert_eq!(
        parsed("review --confirm --resolve flag memo").unwrap().verb,
        Verb::ReviewResolve {
            id: "memo".into(),
            flag: "flag".into(),
            action: ReviewAction::Confirm
        }
    );
    assert_eq!(
        parsed("review memo --resolve flag --dismiss").unwrap().verb,
        Verb::ReviewResolve {
            id: "memo".into(),
            flag: "flag".into(),
            action: ReviewAction::Dismiss
        }
    );
}

#[test]
fn review_resolution_requires_one_flag_and_one_action_without_exposing_corrections() {
    for words in [
        vec!["review", "memo", "--resolve", "flag"],
        vec!["review", "memo", "--confirm"],
        vec!["review", "memo", "--correct", "Private canary"],
        vec![
            "review",
            "memo",
            "--resolve",
            "flag",
            "--confirm",
            "--dismiss",
        ],
        vec![
            "review",
            "memo",
            "--resolve",
            "flag",
            "--confirm",
            "--confirm",
        ],
        vec![
            "review",
            "memo",
            "--resolve",
            "flag",
            "--correct",
            "Private canary",
            "--correct",
            "Other",
        ],
        vec!["review", "memo", "--resolve", "flag", "--correct"],
        vec!["review", "memo", "--resolve", "--json", "--confirm"],
        vec![
            "review",
            "memo",
            "--resolve",
            "flag",
            "--resolve",
            "other",
            "--confirm",
        ],
        vec!["show", "memo", "--correct", "Private canary"],
        vec!["confirm", "--term", "Private canary", "--correct", "Other"],
        vec![
            "review",
            "memo",
            "--resolve",
            "flag",
            "--confirm",
            "--dry-run",
        ],
    ] {
        let error = parse(std::iter::once("vpt").chain(words).map(OsString::from)).unwrap_err();
        assert!(!error.0.contains("Private canary"));
    }
}
