use super::*;

#[test]
fn nested_items_are_separate_claims_and_parent_paragraphs_stay_one_claim() {
    let source = "- parent first.\n\n  parent second.\n  - child first. child second.\n- sibling\n";
    let parsed = extract(source).unwrap();
    assert_eq!(parsed.claims.len(), 3);
    assert_eq!(
        parsed.claims[0].claim.text,
        "parent first.\n\nparent second."
    );
    assert_eq!(parsed.claims[1].claim.text, "child first. child second.");
    assert_eq!(parsed.claims[2].claim.text, "sibling");
    let annotated = parsed.annotate(&vec![Some(Finding::Unsourced); 3]).unwrap();
    assert!(annotated.contains("parent second. [unsourced]\n  - child"));
    assert!(annotated.contains("child first. child second. [unsourced]\n- sibling"));
    let again = extract(&annotated).unwrap();
    assert_eq!(
        again
            .claims
            .iter()
            .map(|c| &c.identity_text)
            .collect::<Vec<_>>(),
        parsed
            .claims
            .iter()
            .map(|c| &c.identity_text)
            .collect::<Vec<_>>()
    );
}

#[test]
fn link_destinations_and_inline_code_do_not_supply_citations_or_annotations() {
    let source = "- [Alice](https://private.test/2040) met [[Person|Bob]]; `[00:00-00:01]` and `[unsourced]` are literal. [00:02-00:03]\n";
    let parsed = extract(source).unwrap();
    let claim = &parsed.claims[0];
    assert_eq!(claim.claim.source_references, ["00:02-00:03"]);
    assert_eq!(
        claim.claim.text,
        "Alice met Bob; [00:00-00:01] and [unsourced] are literal."
    );
    assert!(!claim.identity_text.contains("private.test"));
    let rendered = parsed.annotate(&[None]).unwrap();
    assert_eq!(rendered, source);
}

#[test]
fn escaped_and_entity_encoded_brackets_remain_literal_claim_text() {
    for text in [
        "&#91;unsourced&#93;",
        "\\[unsourced\\]",
        "&#91;00:00-00:01&#93;",
        "\\[00:00-00:01\\]",
        "\\[unsupported: Bob\\]",
    ] {
        let source = format!("- literal {text}\n");
        let parsed = extract(&source).unwrap();
        assert!(parsed.claims[0].claim.source_references.is_empty());
        assert_eq!(parsed.annotate(&[None]).unwrap(), source);
    }
}

#[test]
fn headings_frontmatter_fences_and_indented_code_are_excluded() {
    let source = "\u{feff}---\r\ntitle: Private\r\n---\r\n\r\nSetext Heading\r\n===\r\n\r\n~~~rust\r\n<!-- vpt:links start -->\r\nHidden claim.\r\n~~~\r\n\r\n    Hidden indented code.\r\n\r\n- [x] actual action\r\n";
    let parsed = extract(source).unwrap();
    assert_eq!(parsed.claims.len(), 1);
    assert_eq!(parsed.claims[0].claim.text, "actual action");
    assert!(
        parsed
            .annotate(&[Some(Finding::Unsourced)])
            .unwrap()
            .ends_with("- [x] actual action [unsourced]\r\n")
    );
}

#[test]
fn sentence_splitting_preserves_decimals_abbreviations_quotes_and_unicode() {
    let parsed =
        extract("Dr. Alice paid $1.25. [00:00-00:02] \"Really?\" Yes! 中文。第二句！").unwrap();
    let text: Vec<_> = parsed
        .claims
        .iter()
        .map(|claim| claim.claim.text.as_str())
        .collect();
    assert_eq!(
        text,
        [
            "Dr. Alice paid $1.25.",
            "\"Really?\"",
            "Yes!",
            "中文。",
            "第二句！"
        ]
    );
}

#[test]
fn empty_inverted_and_malformed_time_citations_reach_the_grounder() {
    let parsed = extract("- a [] [00:02-00:01] [00:99-01:00] [00:00-] [not a citation]\n").unwrap();
    assert_eq!(
        parsed.claims[0].claim.source_references,
        ["", "00:02-00:01", "00:99-01:00", "00:00-"]
    );
    assert_eq!(parsed.claims[0].claim.text.trim(), "a     [not a citation]");
}

#[test]
fn transcript_timecodes_are_not_source_ranges_and_blank_ranges_remain_invalid() {
    let parsed = extract("[00:03] Alice spoke.\n\n- another claim [ ]\n").unwrap();
    assert!(parsed.claims[0].claim.source_references.is_empty());
    assert_eq!(parsed.claims[0].claim.text, "[00:03] Alice spoke.");
    assert_eq!(parsed.claims[1].claim.source_references, [" "]);
}

#[test]
fn changed_citation_syntax_changes_claim_identity_even_when_it_displays_the_same() {
    let valid = extract("- Alice [00:00-00:01]\n").unwrap();
    let malformed = extract("- Alice [00&#58;00-00:01]\n").unwrap();
    assert_eq!(valid.claims[0].claim.text, malformed.claims[0].claim.text);
    assert_ne!(
        valid.claims[0].claim.source_references,
        malformed.claims[0].claim.source_references
    );
    assert_ne!(
        valid.claims[0].identity_text,
        malformed.claims[0].identity_text
    );
}

#[test]
fn removing_a_citation_does_not_join_separate_number_or_name_tokens() {
    let parsed = extract("- 1[00:00-00:01]20 and Alice[00:02-00:03]Bob.\n").unwrap();
    assert_eq!(parsed.claims[0].claim.text, "1 20 and Alice Bob.");
    assert_eq!(parsed.claims[0].claim.source_references.len(), 2);
}
