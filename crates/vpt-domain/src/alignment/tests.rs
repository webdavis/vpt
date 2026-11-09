use super::*;

#[test]
fn compares_normalized_words_and_returns_original_word_positions() {
    let actual = compare(&["—", "Straße", "23rd"], &["STRASSE", "!", "23"], 0.35).unwrap();
    assert_eq!(actual.main.dropped_indices, [0]);
    assert_eq!(actual.checker.dropped_indices, [1]);
    assert_eq!(
        actual.outcome,
        Outcome::Aligned {
            distance: 0,
            edits: vec![
                Edit::Equal {
                    main: 1,
                    checker: 0
                },
                Edit::Equal {
                    main: 2,
                    checker: 2
                },
            ]
        }
    );
}

#[test]
fn denominator_is_the_longer_normalized_stream_and_threshold_is_strict() {
    let main = ["one", "two", "three", "four"];
    let checker = ["one", "two", "three"];
    for (left, right) in [(&main[..], &checker[..]), (&checker[..], &main[..])] {
        assert!(matches!(
            compare(left, right, 0.25).unwrap().outcome,
            Outcome::Aligned { distance: 1, .. }
        ));
        assert_eq!(
            compare(left, right, 0.249).unwrap().outcome,
            Outcome::Divergent
        );
    }
    assert!(matches!(
        compare(&["a", "!"], &["a"], 0.1).unwrap().outcome,
        Outcome::Aligned { distance: 0, .. }
    ));
}

#[test]
fn decimal_threshold_equality_is_not_changed_by_multiplication_rounding() {
    let main = vec!["same"; 100];
    let checker = vec!["same"; 71];
    assert!(matches!(
        compare(&main, &checker, 0.29).unwrap().outcome,
        Outcome::Aligned { distance: 29, .. }
    ));
}

#[test]
fn excessive_work_returns_capacity_error_instead_of_inventing_divergence() {
    let main = vec!["first"; 4_000];
    let checker = vec!["second"; 4_000];
    assert_eq!(
        compare(&main, &checker, 1.0),
        Err(AlignmentError::CapacityExceeded)
    );
}

#[test]
fn text_bytes_are_bounded_before_unicode_expansion() {
    let huge = "x".repeat(16 * 1024 * 1024 + 1);
    assert_eq!(
        compare(&[&huge], &[], 0.35),
        Err(AlignmentError::CapacityExceeded)
    );
}

#[test]
fn both_empty_streams_agree_but_nonempty_against_empty_has_distance_one_per_word() {
    assert_eq!(
        compare(&["—"], &["!"], 0.35).unwrap().outcome,
        Outcome::Aligned {
            distance: 0,
            edits: vec![]
        }
    );
    assert_eq!(
        compare(&["one"], &[], 0.35).unwrap().outcome,
        Outcome::Divergent
    );
    assert_eq!(
        compare(&["one"], &[], 1.0).unwrap().outcome,
        Outcome::Aligned {
            distance: 1,
            edits: vec![Edit::Delete { main: 0 }]
        }
    );
}

#[test]
fn replacement_has_myers_insertion_and_deletion_cost() {
    let actual = compare(&["a", "b", "c"], &["a", "x", "c"], 1.0).unwrap();
    assert_eq!(
        actual.outcome,
        Outcome::Aligned {
            distance: 2,
            edits: vec![
                Edit::Equal {
                    main: 0,
                    checker: 0
                },
                Edit::Delete { main: 1 },
                Edit::Insert { checker: 1 },
                Edit::Equal {
                    main: 2,
                    checker: 2
                },
            ]
        }
    );
}

#[test]
fn invalid_ratios_and_excess_input_return_typed_errors() {
    for ratio in [f64::NAN, f64::INFINITY, -0.1, 0.0, 1.01] {
        assert_eq!(compare(&[], &[], ratio), Err(AlignmentError::InvalidRatio));
    }
    assert_eq!(
        compare(&vec!["a"; 65_537], &[], 0.35),
        Err(AlignmentError::CapacityExceeded)
    );
}

#[test]
fn exhaustive_small_streams_match_reference_distance_and_reconstruct_inputs() {
    let streams: Vec<Vec<&str>> = (0..32)
        .map(|bits| {
            let length = if bits == 0 {
                0
            } else {
                32 - (bits as u32).leading_zeros()
            };
            (0..length)
                .map(|i| if bits & (1 << i) == 0 { "a" } else { "b" })
                .collect()
        })
        .collect();
    for main in &streams {
        for checker in &streams {
            let expected = reference_distance(main, checker);
            let result = compare(main, checker, 1.0).unwrap();
            if expected > main.len().max(checker.len()) {
                assert_eq!(result.outcome, Outcome::Divergent);
                continue;
            }
            let Outcome::Aligned { distance, edits } = result.outcome else {
                panic!("unexpected divergence")
            };
            assert_eq!(distance, expected, "{main:?} -> {checker:?}");
            let (mut a, mut b) = (0, 0);
            for edit in edits {
                match edit {
                    Edit::Equal {
                        main: i,
                        checker: j,
                    } => {
                        assert_eq!((i, j), (a, b));
                        assert_eq!(main[i], checker[j]);
                        a += 1;
                        b += 1;
                    }
                    Edit::Delete { main: i } => {
                        assert_eq!(i, a);
                        a += 1;
                    }
                    Edit::Insert { checker: j } => {
                        assert_eq!(j, b);
                        b += 1;
                    }
                }
            }
            assert_eq!((a, b), (main.len(), checker.len()));
        }
    }
}

fn reference_distance(a: &[&str], b: &[&str]) -> usize {
    let mut row: Vec<_> = (0..=b.len()).collect();
    for (i, left) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, right) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if left == right {
                diagonal
            } else {
                (above + 1).min(row[j] + 1)
            };
            diagonal = above;
        }
    }
    row[b.len()]
}
