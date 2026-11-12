use super::*;
use crate::proposal::Due;

fn finding(spoken: &str) -> Option<Finding> {
    let words = words(&[("send", 0.0, 0.5), (spoken, 0.5, 1.0), ("Friday", 1.5, 2.0)]);
    let due = Due {
        date: "2026-11-19".into(),
        time: None,
        source_references: vec!["00:00-00:01".into()],
    };
    Grounder::new(words.iter(), &[], 4.0, 1)
        .unwrap()
        .due(&due)
        .unwrap()
        .finding
}

#[test]
fn a_cited_time_expression_keeps_the_due_date() {
    for spoken in [
        "Thursday",
        "THU",
        "Friday's",
        "Dec",
        "3pm",
        "15:00",
        "Twenty-First",
        "tomorrow,",
        "EOD",
        "weeks",
        "deadline",
        "٣",
    ] {
        assert_eq!(finding(spoken), None, "{spoken}");
    }
}

#[test]
fn cited_words_without_a_time_withhold_the_due_date() {
    for spoken in ["notes", "weekly", "Thursdays", "Sept.", "soon", "jueves"] {
        assert_eq!(
            finding(spoken),
            Some(Finding::Unsupported("2026-11-19".into())),
            "{spoken}"
        );
    }
}
