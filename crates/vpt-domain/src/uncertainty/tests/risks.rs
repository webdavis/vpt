use super::*;

#[test]
fn repeated_names_are_one_flag_with_every_occurrence() {
    let result = classify(
        &transcript("we saw Alice then Alice left"),
        None,
        Options::default(),
        &[],
    )
    .unwrap();
    assert_eq!(classes(&result), [Class::AgreedUnverified]);
    assert_eq!(result.lexical[0].record_text, "Alice");
    assert_eq!(result.lexical[0].ranges, [2..3, 4..5]);
}

#[test]
fn multiword_names_are_whole_spans_and_known_terms_keep_punctuation() {
    let input = transcript("we saw New York then New York. then York");
    let result = classify(&input, None, Options::default(), &["new york"]).unwrap();
    assert_eq!(
        result
            .lexical
            .iter()
            .map(|flag| flag.record_text.as_str())
            .collect::<Vec<_>>(),
        ["New York.", "York"]
    );
    assert_eq!(result.lexical[0].ranges, [5..7]);
}

#[test]
fn confidence_floor_is_strict_and_wins_over_risk_and_known_terms() {
    let mut input = transcript("we saw Alice Bob Carol Dave");
    for (word, confidence) in
        input.segments[0].words[2..]
            .iter_mut()
            .zip([Some(0.349), Some(0.35), None, Some(0.9)])
    {
        word.confidence = confidence;
    }
    let result = classify(&input, None, Options::default(), &["Alice Bob Carol Dave"]).unwrap();
    assert_eq!(classes(&result), [Class::LowConfidence]);
    assert_eq!(result.lexical[0].ranges, [2..3]);
    assert_eq!(result.lexical[0].confidence, Some(0.349));
}

#[test]
fn numbers_dates_times_and_money_are_risky_without_a_checker() {
    for text in [
        "we counted 42",
        "we meet 2026-10-01",
        "we meet 12:30",
        "we paid $42.50",
        "we counted twenty three",
        "we meet monday",
        "we paid five dollars",
    ] {
        let result = classify(&transcript(text), None, Options::default(), &[]).unwrap();
        assert_eq!(classes(&result), [Class::AgreedUnverified], "{text}");
        assert_eq!(
            result.lexical[0].ranges,
            [2..text.split_whitespace().count()]
        );
    }
}

#[test]
fn same_surface_with_different_capitalization_is_not_aggregated() {
    let result = classify(
        &transcript("we saw Alice then ALICE"),
        None,
        Options::default(),
        &[],
    )
    .unwrap();
    assert_eq!(result.lexical.len(), 2);
}

#[test]
fn known_term_suppresses_only_full_risk_spans_never_substrings() {
    let result = classify(
        &transcript("we saw Ann then Anna then Ann,"),
        None,
        Options::default(),
        &["ann"],
    )
    .unwrap();
    assert_eq!(
        result
            .lexical
            .iter()
            .map(|flag| flag.record_text.as_str())
            .collect::<Vec<_>>(),
        ["Anna", "Ann,"]
    );
}

#[test]
fn debug_output_does_not_expose_record_or_alternative_text() {
    let result = compared(
        "we saw SecretName yesterday",
        "we saw OtherSecret yesterday",
    );
    let debug = format!("{result:?}");
    assert!(!debug.contains("Secret"));
    assert!(debug.contains("ProperNoun"));
}

#[test]
fn known_term_inside_a_larger_name_is_never_flagged_as_unverified() {
    let result = classify(
        &transcript("we saw Alice Bob"),
        None,
        Options::default(),
        &["Alice"],
    )
    .unwrap();
    assert_eq!(result.lexical.len(), 1);
    assert_eq!(result.lexical[0].record_text, "Bob");
    assert_eq!(result.lexical[0].ranges, [3..4]);
}

#[test]
fn spelled_ordinals_and_large_quantities_are_unverified_risk_spans() {
    for number in [
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
        "twentieth",
        "thirtieth",
        "fortieth",
        "fiftieth",
        "sixtieth",
        "seventieth",
        "eightieth",
        "ninetieth",
        "hundredth",
        "thousandth",
        "millionth",
        "billionth",
        "trillion",
        "trillionth",
        "twenty-first",
        "one-hundredth",
    ] {
        let input = transcript(&format!("we counted {number}"));
        for checker in [None, Some(&input)] {
            let result = classify(&input, checker, Options::default(), &[]).unwrap();
            assert_eq!(classes(&result), [Class::AgreedUnverified], "{number}");
            assert_eq!(result.lexical[0].record_text, number);
            assert_eq!(result.lexical[0].ranges, [2..3]);
            assert!(
                classify(&input, checker, Options::default(), &[number])
                    .unwrap()
                    .lexical
                    .is_empty()
            );
        }
    }
}
