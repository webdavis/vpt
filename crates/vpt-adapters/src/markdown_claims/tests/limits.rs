use super::*;

#[test]
fn oversized_claims_and_excessive_citations_are_refused() {
    let text = format!("- {}\n", "x".repeat(65_537));
    assert!(matches!(extract(&text), Err(ClaimError::CapacityExceeded)));
    let text = format!("- claim {}\n", "[00:00-00:01] ".repeat(257));
    assert!(matches!(extract(&text), Err(ClaimError::CapacityExceeded)));
}

#[test]
fn deep_structure_and_excessive_syntax_are_bounded_before_annotation() {
    let text = format!("{}claim", "> ".repeat(129));
    assert!(matches!(extract(&text), Err(ClaimError::CapacityExceeded)));
    let text = "*".repeat(1_048_577);
    assert!(matches!(extract(&text), Err(ClaimError::CapacityExceeded)));
}

#[test]
fn annotation_count_and_payload_limits_are_checked() {
    let parsed = extract("- one\n- two\n").unwrap();
    assert_eq!(
        parsed.annotate(&[None]),
        Err(ClaimError::InvalidAnnotations)
    );
    assert_eq!(
        parsed.annotate(&[Some(Finding::Unsupported("x".repeat(65_537))), None]),
        Err(ClaimError::CapacityExceeded)
    );
}

#[test]
fn excessive_sentence_count_is_refused_and_long_dotted_words_do_not_split() {
    let source = "word. ".repeat(65_537);
    assert!(matches!(
        extract(&source),
        Err(ClaimError::CapacityExceeded)
    ));
    let source = format!("{}end", "a.".repeat(30_000));
    assert_eq!(extract(&source).unwrap().claims.len(), 1);
}

#[test]
fn altered_public_offsets_are_refused_without_panicking() {
    let mut parsed = extract("- é\n").unwrap();
    parsed.claims[0].end = 3;
    assert_eq!(
        parsed.annotate(&[Some(Finding::Unsourced)]),
        Err(ClaimError::InvalidAnnotations)
    );
}
