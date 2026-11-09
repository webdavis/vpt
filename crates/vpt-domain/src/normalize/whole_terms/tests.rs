use super::*;

fn matches(words: &[&str], terms: &[&str]) -> Vec<(usize, Range<usize>)> {
    let mut result = vec![];
    TermMatcher::new(terms)
        .unwrap()
        .scan(words, |term, range| {
            result.push((term, range));
            Ok(())
        })
        .unwrap();
    result
}

#[test]
fn matches_whole_tokens_with_case_folding_and_preserves_punctuation() {
    assert_eq!(
        matches(
            &["Annette", "Ann,", "ANN", "C++", "c", "Straße", "STRASSE"],
            &["ann", "C++", "strasse"]
        ),
        [(0, 2..3), (1, 3..4), (2, 5..6), (2, 6..7)]
    );
}

#[test]
fn matches_all_overlapping_phrases_and_deduplicates_equivalent_terms() {
    assert_eq!(
        matches(
            &["New", "York", "NEW", "york"],
            &["New", "New York", " NEW   YORK ", "York"]
        ),
        [
            (0, 0..1),
            (1, 0..2),
            (3, 1..2),
            (0, 2..3),
            (1, 2..4),
            (3, 3..4)
        ]
    );
    assert!(matches(&["test"], &["", "  "]).is_empty());
}

#[test]
fn refuses_oversized_inputs_and_hides_private_terms_in_debug_output() {
    assert_eq!(
        TermMatcher::new(&vec!["a"; 65_537]).unwrap_err(),
        MatchError::CapacityExceeded
    );
    assert_eq!(
        TermMatcher::new(&[&"a".repeat(65_537)]).unwrap_err(),
        MatchError::CapacityExceeded
    );
    let matcher = TermMatcher::new(&["Private canary"]).unwrap();
    assert!(!format!("{matcher:?}").contains("Private"));
    assert_eq!(
        matcher.scan(&vec!["a"; 65_537], |_, _| Ok(())),
        Err(MatchError::CapacityExceeded)
    );
}

#[test]
fn consumer_capacity_refusal_stops_matching() {
    let mut visits = 0;
    let result = TermMatcher::new(&["a"]).unwrap().scan(&["a", "a"], |_, _| {
        visits += 1;
        Err(MatchError::CapacityExceeded)
    });
    assert_eq!(result, Err(MatchError::CapacityExceeded));
    assert_eq!(visits, 1);
}

#[test]
fn traversal_work_is_bounded_even_when_many_prefixes_match() {
    let matcher = TermMatcher::new(&["a a a a a a a a a a"]).unwrap();
    assert_eq!(
        matcher.scan_with(&["a"; 100], |_, _| Ok(()), 20),
        Err(MatchError::CapacityExceeded)
    );
}
