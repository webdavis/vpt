use super::*;
use vpt_domain::review::ReviewAction;

#[test]
fn a_correction_is_taken_literally_kept_out_of_debug_and_beside_global_options() {
    for text in [
        "Private canary",
        "-x private",
        "--term",
        "--correct",
        "--resolve",
        "--confirm",
    ] {
        let invocation = parse(
            [
                "vpt",
                "--json",
                "review",
                "correct",
                "memo",
                "flag",
                text,
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
}

#[test]
fn each_review_act_is_its_own_subcommand() {
    for (line, action) in [
        ("review confirm memo flag", ReviewAction::Confirm),
        ("review dismiss memo flag --json", ReviewAction::Dismiss),
    ] {
        assert_eq!(
            parsed(line).unwrap().verb,
            Verb::ReviewResolve {
                id: "memo".into(),
                flag: "flag".into(),
                action
            },
            "{line}"
        );
    }
    assert_eq!(
        parsed("review list memo").unwrap().verb,
        Verb::Review { id: "memo".into() }
    );
}

#[test]
fn bare_review_shows_help_and_the_removed_flags_are_refused() {
    let Verb::Print(help) = parsed("review").unwrap().verb else {
        panic!("help");
    };
    assert!(help.contains("correct"), "{help}");
    for line in [
        "review memo",
        "review memo --resolve flag --confirm",
        "review list memo --resolve flag --dismiss",
        "review confirm memo",
        "review correct memo flag",
        "review dismiss memo flag extra",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}

#[test]
fn a_refused_correction_never_shows_its_text() {
    for words in [
        vec!["review", "correct", "memo", "Private canary"],
        vec![
            "review",
            "correct",
            "memo",
            "flag",
            "Private canary",
            "extra",
        ],
        vec![
            "review",
            "correct",
            "memo",
            "flag",
            "Private canary",
            "--preview",
        ],
        vec!["review", "confirm", "memo", "flag", "Private canary"],
        vec!["show", "memo", "Private canary"],
    ] {
        let error = parse(std::iter::once("vpt").chain(words).map(OsString::from)).unwrap_err();
        assert!(!format!("{error:?}").contains("Private canary"));
    }
}
