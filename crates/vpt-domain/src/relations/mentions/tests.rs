use super::*;

fn term(text: &str, lookup: Lookup) -> ConfirmedTerm<'_> {
    ConfirmedTerm { text, lookup }
}

#[test]
fn unique_whole_token_terms_keep_every_occurrence_with_unicode_case_folding() {
    let result = mentions(
        &[
            "Annette", "ANN", "Ann,", "New", "York", "new", "york", "Straße", "STRASSE",
        ],
        &[
            term("ann", Lookup::Unique(3)),
            term("New York", Lookup::Unique(1)),
            term("strasse", Lookup::Unique(2)),
        ],
    )
    .unwrap();
    assert_eq!(
        result.links,
        [
            Mention {
                target: 1,
                ranges: vec![3..5, 5..7]
            },
            Mention {
                target: 2,
                ranges: vec![7..8, 8..9]
            },
            Mention {
                target: 3,
                ranges: std::iter::once(1..2).collect()
            },
        ]
    );
    assert!(result.ambiguous.is_empty());
}

#[test]
fn ambiguous_matches_are_recorded_without_links_and_missing_targets_are_ignored() {
    let result = mentions(
        &["Ann", "Ann", "unknown"],
        &[
            term("Ann", Lookup::Ambiguous),
            term("unknown", Lookup::Missing),
            term("absent", Lookup::Ambiguous),
        ],
    )
    .unwrap();
    assert!(result.links.is_empty());
    assert_eq!(
        result.ambiguous,
        [AmbiguousMention {
            term: 0,
            ranges: vec![0..1, 1..2]
        }]
    );
}

#[test]
fn aliases_of_one_note_merge_ranges_without_losing_nested_matches() {
    let result = mentions(
        &["New", "York", "NYC"],
        &[
            term("New York", Lookup::Unique(1)),
            term("New", Lookup::Unique(1)),
            term("NYC", Lookup::Unique(1)),
            term("new york", Lookup::Unique(1)),
        ],
    )
    .unwrap();
    assert_eq!(
        result.links,
        [Mention {
            target: 1,
            ranges: vec![0..1, 0..2, 2..3]
        }]
    );
}

#[test]
fn punctuation_is_part_of_confirmed_terms_and_debug_output_omits_private_text() {
    let terms = [term("Private C++", Lookup::Unique(0))];
    let result = mentions(&["private", "c++", "private", "c"], &terms).unwrap();
    assert_eq!(
        result.links,
        [Mention {
            target: 0,
            ranges: std::iter::once(0..2).collect()
        }]
    );
    assert!(!format!("{:?} {result:?}", terms[0]).contains("Private"));
}

#[test]
fn excessive_retained_ranges_are_refused() {
    let words = vec!["a"; 65_536];
    let terms = [term("a", Lookup::Unique(0)), term("a a", Lookup::Unique(0))];
    assert_eq!(mentions(&words, &terms), Err(MatchError::CapacityExceeded));
}
