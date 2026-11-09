use super::*;

#[test]
fn tag_actions_keep_private_values_literal_and_require_a_recording() {
    for flag in ["--tag", "--reject-tag"] {
        for value in ["Private topic", "--json", "--tag"] {
            let invocation =
                parse(["vpt", "confirm", "recording", flag, value, "--json"].map(OsString::from))
                    .unwrap();
            assert!(invocation.json);
            assert!(!format!("{invocation:?}").contains(value));
            let Verb::ConfirmTag { id, tag, action } = invocation.verb else {
                panic!("tag");
            };
            assert_eq!(id, "recording");
            assert_eq!(tag.as_str(), value);
            assert_eq!(
                action,
                if flag == "--tag" {
                    vpt_domain::tags::Action::Confirm
                } else {
                    vpt_domain::tags::Action::Reject
                }
            );
        }
    }
    for args in [
        vec!["confirm", "--tag", "Private topic"],
        vec![
            "confirm",
            "id",
            "--tag",
            "Private topic",
            "--reject-tag",
            "another",
        ],
        vec![
            "confirm",
            "id",
            "--tag",
            "Private topic",
            "--term",
            "another",
        ],
        vec!["show", "id", "--tag", "Private topic"],
        vec!["confirm", "id", "--tag", "Private topic", "--dry-run"],
    ] {
        let error = parse(std::iter::once("vpt").chain(args).map(OsString::from)).unwrap_err();
        assert!(!error.0.contains("Private topic"));
    }
}

#[test]
fn confirmation_keeps_text_values_private_and_separate_from_global_options() {
    for term in ["Private canary", "--json", "--config", "--term"] {
        let invocation = parse(
            [
                "vpt",
                "--json",
                "confirm",
                "--term",
                term,
                "--config",
                "settings.toml",
            ]
            .map(OsString::from),
        )
        .unwrap();
        assert!(invocation.json);
        assert_eq!(invocation.config, Some("settings.toml".into()));
        assert!(!format!("{invocation:?}").contains(term));
        let Verb::ConfirmTerm { term: parsed } = invocation.verb else {
            panic!("confirm")
        };
        assert_eq!(parsed.as_str(), term);
    }
}

#[test]
fn confirmation_requires_one_term_and_refuses_unrelated_options() {
    for words in [
        vec!["vpt", "confirm"],
        vec!["vpt", "confirm", "--term"],
        vec![
            "vpt",
            "confirm",
            "--term",
            "Private canary",
            "--term",
            "Other",
        ],
        vec!["vpt", "confirm", "--term", "Private canary", "--dry-run"],
        vec!["vpt", "show", "id", "--term", "Private canary"],
    ] {
        let error = parse(words.into_iter().map(OsString::from)).unwrap_err();
        assert!(!error.0.contains("Private canary"));
    }
}
