use super::*;

#[test]
fn compatibility_normalization_and_full_case_fold_share_one_policy() {
    assert_eq!(canonical("Straße STRASSE"), "strasse strasse");
    assert_eq!(canonical("ΟΣ ος οσ"), "οσ οσ οσ");
    assert_eq!(canonical("ＦＯＯ ﬃ K"), "foo ffi k");
    assert_eq!(canonical("Cafe\u{301}"), canonical("CAFÉ"));
}

#[test]
fn alignment_drops_punctuation_joins_digits_and_normalizes_ordinals_with_source_mapping() {
    let normalized = alignment(&[
        "—",
        "12,400",
        "23rd",
        "2ND",
        "1st",
        "4th",
        "Smith-Jones",
        "!",
        "First",
    ]);
    let values: Vec<_> = normalized
        .tokens
        .iter()
        .map(|token| (token.original_index, token.text.as_str()))
        .collect();
    assert_eq!(
        values,
        vec![
            (1, "12400"),
            (2, "23"),
            (3, "2"),
            (4, "1"),
            (5, "4"),
            (6, "smithjones"),
            (8, "first")
        ]
    );
    assert_eq!(normalized.dropped_indices, vec![0, 7]);
}

#[test]
fn ordinal_suffixes_require_a_digit_prefix_and_no_number_word_conversion() {
    let normalized = alignment(&[
        "first",
        "twenty-three",
        "23abc",
        "1station",
        "north",
        "23rd",
    ]);
    let values: Vec<_> = normalized
        .tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect();
    assert_eq!(
        values,
        ["first", "twentythree", "23abc", "1station", "north", "23"]
    );
}

#[test]
fn known_terms_match_whole_intact_tokens_and_keep_every_occurrence() {
    let words = [
        "Joann",
        "ANN",
        "C++",
        "C",
        "Smith-Jones",
        "SmithJones",
        "Ann",
        "New",
        "York",
        "Newark",
    ];
    assert_eq!(whole_term_matches(&words, "ann"), vec![1..2, 6..7]);
    assert_eq!(whole_term_matches(&words, "C++"), vec![2..3]);
    assert_eq!(whole_term_matches(&words, "smith-jones"), vec![4..5]);
    assert_eq!(whole_term_matches(&words, "new york"), vec![7..9]);
    assert_eq!(
        whole_term_matches(&["Straße", "STRASSE"], "strasse"),
        vec![0..1, 1..2]
    );
    assert!(whole_term_matches(&words, "  ").is_empty());
}

#[test]
fn normalization_keeps_original_input_unchanged() {
    let words = ["23rd", "ＦＯＯ", "12,400"];
    alignment(&words);
    assert_eq!(words, ["23rd", "ＦＯＯ", "12,400"]);
}
