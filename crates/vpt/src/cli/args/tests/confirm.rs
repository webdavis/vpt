use super::*;
use vpt_domain::tags::Action;

const LITERAL: [&str; 5] = ["Private topic", "-x", "--reject-tag", "--tag", "--term"];

#[test]
fn tag_acts_keep_private_values_literal_and_out_of_debug() {
    for (verb, expected) in [("tag", Action::Confirm), ("reject-tag", Action::Reject)] {
        for value in LITERAL {
            let invocation =
                parse(["vpt", "confirm", verb, "recording", value, "--json"].map(OsString::from))
                    .unwrap();
            assert_eq!(invocation.format, Some(Format::Json));
            assert!(!format!("{invocation:?}").contains(value));
            let Verb::ConfirmTag { id, tag, action } = invocation.verb else {
                panic!("tag");
            };
            assert_eq!(
                (id.as_str(), tag.as_str(), action),
                ("recording", value, expected)
            );
        }
    }
}

#[test]
fn a_term_is_taken_literally_beside_global_options() {
    for term in LITERAL {
        let invocation = parse(
            [
                "vpt",
                "--json",
                "confirm",
                "term",
                term,
                "--config",
                "settings.toml",
            ]
            .map(OsString::from),
        )
        .unwrap();
        assert_eq!(invocation.format, Some(Format::Json));
        assert_eq!(invocation.config, Some("settings.toml".into()));
        assert!(!format!("{invocation:?}").contains(term));
        let Verb::ConfirmTerm { term: parsed } = invocation.verb else {
            panic!("confirm")
        };
        assert_eq!(parsed.as_str(), term);
    }
}

#[test]
fn a_value_spelled_like_a_global_flag_follows_the_end_of_flags() {
    let invocation =
        parse(["vpt", "--json", "confirm", "term", "--", "--json"].map(OsString::from)).unwrap();
    let Verb::ConfirmTerm { term } = invocation.verb else {
        panic!("confirm")
    };
    assert_eq!(term.as_str(), "--json");
}

#[test]
fn bare_confirm_shows_help_and_the_removed_flags_are_refused() {
    let Verb::Print(help) = parsed("confirm").unwrap().verb else {
        panic!("help");
    };
    assert!(help.contains("reject-tag"), "{help}");
    for line in [
        "confirm --term word",
        "confirm memo --tag word",
        "confirm memo --reject-tag word",
        "confirm memo --relation related:x",
        "confirm term",
        "confirm tag memo",
        "confirm term word --preview",
    ] {
        assert!(parsed(line).is_err(), "{line}");
    }
}

#[test]
fn a_refused_confirmation_never_shows_its_value() {
    for words in [
        vec!["confirm", "term", "Private canary", "Other"],
        vec!["confirm", "tag", "Private canary"],
        vec!["confirm", "tag", "id", "Private canary", "Other"],
        vec!["confirm", "reject-tag", "id", "Private canary", "--preview"],
        vec!["confirm", "Private canary"],
        vec!["show", "id", "--term", "Private canary"],
    ] {
        let error = parse(std::iter::once("vpt").chain(words).map(OsString::from)).unwrap_err();
        assert!(
            !format!("{error:?}").contains("Private canary"),
            "{error:?}"
        );
    }
}
