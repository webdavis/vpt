use super::*;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn select(values: &[&str], known: &[&str], options: Options) -> Selection {
    let vocabulary = Vocabulary::new(&strings(known)).unwrap();
    vocabulary.select(&strings(values), options).unwrap()
}

#[test]
fn hold_reuses_existing_spelling_and_normalizes_new_multiword_tags() {
    let result = select(
        &["PROJECT finance", "new  topic", "Straße", "cafe\u{301}"],
        &["Project_Finance", "STRASSE", "Café"],
        Options::default(),
    );
    assert_eq!(
        result.confirmed,
        strings(&["Project_Finance", "STRASSE", "Café"])
    );
    assert_eq!(result.suggested, strings(&["new-topic"]));
    assert!(result.learn.is_empty());
    assert!(result.rejected.is_empty());
}

#[test]
fn write_confirms_and_learns_only_retained_new_tags_in_proposal_order() {
    let result = select(
        &["Fresh Topic", "known", "fresh_topic", "second", "third"],
        &["Known"],
        Options {
            new_tags: NewTags::Write,
            max_per_note: 3,
            ..Options::default()
        },
    );
    assert_eq!(
        result.confirmed,
        strings(&["fresh-topic", "Known", "second"])
    );
    assert_eq!(result.learn, strings(&["fresh-topic", "second"]));
    assert!(result.suggested.is_empty());
    assert_eq!(
        result.rejected,
        vec![Rejected {
            index: 4,
            reason: Rejection::ConfirmedLimit
        }]
    );
}

#[test]
fn limits_are_independent_and_duplicate_proposals_do_not_consume_them() {
    let result = select(
        &[
            "first",
            "FIRST",
            "second",
            "unknown one",
            "UNKNOWN_ONE",
            "unknown two",
        ],
        &["first", "second"],
        Options {
            max_per_note: 1,
            max_suggested: 1,
            ..Options::default()
        },
    );
    assert_eq!(result.confirmed, strings(&["first"]));
    assert_eq!(result.suggested, strings(&["unknown-one"]));
    assert_eq!(
        result.rejected,
        vec![
            Rejected {
                index: 2,
                reason: Rejection::ConfirmedLimit
            },
            Rejected {
                index: 5,
                reason: Rejection::SuggestedLimit
            },
        ]
    );
}

#[test]
fn invalid_syntax_is_rejected_before_classification_even_in_write_mode() {
    let proposals = [
        "#hidden",
        "123",
        "",
        "bad.tag",
        "a\u{0}b",
        "[private]",
        "１２３",
        "year2026",
        "中文",
        "project/finance",
    ];
    let result = select(
        &proposals,
        &[],
        Options {
            new_tags: NewTags::Write,
            ..Options::default()
        },
    );
    assert_eq!(
        result.confirmed,
        strings(&["year2026", "中文", "project/finance"])
    );
    assert_eq!(
        result.rejected,
        (0..7)
            .map(|index| Rejected {
                index,
                reason: Rejection::Syntax
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn equivalent_vocabulary_entries_use_the_first_existing_spelling() {
    let result = select(
        &["project-finance"],
        &["Project/Finance", "project_finance"],
        Options::default(),
    );
    assert_eq!(result.confirmed, strings(&["Project/Finance"]));
}

#[test]
fn zero_limits_reject_all_unique_tags_without_learning_them() {
    let result = select(
        &["known", "new"],
        &["known"],
        Options {
            max_per_note: 0,
            max_suggested: 0,
            ..Options::default()
        },
    );
    assert!(result.confirmed.is_empty());
    assert!(result.suggested.is_empty());
    assert!(result.learn.is_empty());
    assert_eq!(result.rejected.len(), 2);
}

#[test]
fn untrusted_input_is_bounded_and_debug_output_omits_tag_text() {
    let vocabulary = Vocabulary::new(&strings(&["PrivateCanary"])).unwrap();
    assert!(!format!("{vocabulary:?}").contains("PrivateCanary"));
    let selected = vocabulary
        .select(
            &strings(&["PrivateCanary", "PrivateSuggestion"]),
            Options::default(),
        )
        .unwrap();
    assert!(!format!("{selected:?}").contains("Private"));
    assert_eq!(
        vocabulary.select(&vec!["tag".into(); 257], Options::default()),
        Err(TagError::CapacityExceeded)
    );
    assert_eq!(
        Vocabulary::new(&vec!["tag".into(); 65_537]).unwrap_err(),
        TagError::CapacityExceeded
    );
    assert_eq!(
        vocabulary.select(&["a".repeat(65_537)], Options::default()),
        Err(TagError::CapacityExceeded)
    );
}

#[test]
fn malformed_vocabulary_is_refused_without_exposing_its_text() {
    assert_eq!(
        Vocabulary::new(&strings(&["Private#Canary"])).unwrap_err(),
        TagError::InvalidVocabulary
    );
}
