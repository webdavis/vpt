use super::*;

#[test]
fn numeric_disagreements_rank_before_names_and_other_words() {
    let result = compared(
        "we paid 42 to Alice for blue paint today",
        "we paid 43 to Bob for green paint today",
    );
    assert_eq!(
        classes(&result),
        [Class::Numeric, Class::ProperNoun, Class::Other]
    );
    assert_eq!(result.lexical[0].ranges, [2..3]);
    assert_eq!(result.lexical[0].record_text, "42");
    assert_eq!(result.lexical[0].alternative_text.as_deref(), Some("43"));
    assert_eq!(result.lexical[1].ranges, [4..5]);
}

#[test]
fn formatting_insertions_and_ordinals_are_recorded_without_surfacing() {
    let result = compared("we met on 23rd of May", "we met on the 23 of May");
    assert_eq!(
        classes(&result),
        [
            Class::Formatting,
            Class::Formatting,
            Class::AgreedUnverified
        ]
    );
    assert_eq!(result.lexical[0].ranges, [3..3]);
    assert_eq!(result.lexical[0].alternative_text.as_deref(), Some("the"));
    assert_eq!(result.lexical[1].ranges, [3..4]);
    assert!(!result.lexical[0].class.surfaced());
    assert!(!result.lexical[1].class.surfaced());
}

#[test]
fn insertions_and_deletions_retain_the_exact_gap_or_record_range() {
    for (main, checker, range, record, alternative) in [
        ("we go home", "we go directly home", 2..2, "", "directly"),
        ("we go directly home", "we go home", 2..3, "directly", ""),
        ("we go home", "we go home tomorrow", 3..3, "", "tomorrow"),
    ] {
        let result = compared(main, checker);
        assert_eq!(classes(&result), [Class::Other]);
        assert_eq!(result.lexical[0].ranges, [range]);
        assert_eq!(result.lexical[0].record_text, record);
        assert_eq!(
            result.lexical[0].alternative_text.as_deref(),
            Some(alternative)
        );
    }
}

#[test]
fn sentence_initial_capitals_do_not_make_proper_noun_disagreements() {
    let result = compared(
        "Hello there. Welcome back again",
        "Goodbye there. Greetings back again",
    );
    assert_eq!(classes(&result), [Class::Other, Class::Other]);
    let result = compared("we met alice yesterday", "we met Bob yesterday");
    assert_eq!(classes(&result), [Class::ProperNoun]);
}

#[test]
fn disagreement_takes_precedence_over_low_confidence_and_known_terms() {
    let mut record = transcript("we met Alice yesterday");
    record.segments[0].words[2].confidence = Some(0.1);
    let result = classify(
        &record,
        Some(&transcript("we met Bob yesterday")),
        Options {
            max_divergence_ratio: 1.0,
            ..Options::default()
        },
        &["Alice"],
    )
    .unwrap();
    assert_eq!(classes(&result), [Class::ProperNoun]);
}

#[test]
fn divergence_removes_span_disagreements_but_keeps_record_risk_and_confidence() {
    let mut record = transcript("we paid Alice 42");
    record.segments[0].words[1].confidence = Some(0.1);
    let result = classify(
        &record,
        Some(&transcript("different content entirely elsewhere")),
        Options::default(),
        &[],
    )
    .unwrap();
    assert_eq!(result.diagnostics, [Diagnostic::Divergent]);
    assert_eq!(
        classes(&result),
        [
            Class::LowConfidence,
            Class::AgreedUnverified,
            Class::AgreedUnverified
        ]
    );
    assert!(
        result
            .lexical
            .iter()
            .all(|flag| flag.alternative_text.is_none())
    );
}

#[test]
fn punctuation_and_case_normalize_without_inventing_disagreement() {
    let result = compared("we paid 12,400 for Straße", "WE PAID 12400 FOR STRASSE");
    assert_eq!(
        classes(&result),
        [Class::AgreedUnverified, Class::AgreedUnverified]
    );
    assert_eq!(result.lexical[0].record_text, "12,400");
    assert_eq!(result.lexical[1].record_text, "Straße");
}
