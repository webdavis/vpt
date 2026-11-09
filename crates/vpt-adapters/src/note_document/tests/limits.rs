use super::*;

#[test]
fn deeply_nested_or_oversized_frontmatter_is_refused() {
    let nested = format!("custom: {}1{}\n", "[".repeat(65), "]".repeat(65));
    assert!(matches!(
        Document::parse(&document(&nested)),
        Err(NoteError::CapacityExceeded)
    ));
    let text = "x".repeat(65_537);
    assert!(matches!(
        Document::parse(&document(&format!("custom: {text}\n"))),
        Err(NoteError::CapacityExceeded)
    ));
    let tags = format!("tags: [{}]\n", vec!["tag"; 257].join(", "));
    assert!(matches!(
        Document::parse(&document(&tags)),
        Err(NoteError::CapacityExceeded)
    ));
}

#[test]
fn cyclic_and_exponential_aliases_are_bounded_during_rendering() {
    let source = document("");
    let parsed = Document::parse(&source).unwrap();
    let cycle = document("vptExtra: &cycle [*cycle]\n");
    assert!(matches!(
        parsed.rewrite(&cycle, &[]),
        Err(NoteError::CapacityExceeded)
    ));
    let mut extra = String::from("a0: &a0 [word]\n");
    for index in 1..20 {
        extra.push_str(&format!(
            "a{index}: &a{index} [*a{}, *a{}]\n",
            index - 1,
            index - 1
        ));
    }
    extra.push_str("vptExtra: *a19\n");
    assert!(matches!(
        parsed.rewrite(&document(&extra), &[]),
        Err(NoteError::CapacityExceeded)
    ));
}

#[test]
fn malformed_repeated_documents_and_unsupported_schema_types_are_refused() {
    for extra in ["tags: [one\n", "? [complex, key]\n: value\n"] {
        assert!(Document::parse(&document(extra)).is_err());
    }
    assert!(Document::parse(&document("...\n---\nother: operator prose\n")).is_ok());
    for schema in ["!!str 1", "1.0", "-1", "4294967296", "[1]"] {
        assert!(
            Document::parse(&document("").replace("vptSchema: 1", &format!("vptSchema: {schema}")))
                .is_err()
        );
    }
}
