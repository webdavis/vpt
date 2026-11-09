use super::*;

#[test]
fn correction_annotations_preserve_original_claim_identity_and_citations() {
    let original = "- Alice [00:00-00:01]\n";
    let corrected = "- Alice [00:00-00:01] [corrected: Bob &#91;00:02-00:03&#93;&#10;next]\n";
    let before = extract(original).unwrap();
    let after = extract(corrected).unwrap();
    assert_eq!(after.claims.len(), 1);
    assert_eq!(
        after.claims[0].identity_text,
        before.claims[0].identity_text
    );
    assert_eq!(after.claims[0].claim, before.claims[0].claim);
    assert_eq!(after.annotate(&[None]).unwrap(), original);
    let literal = "- `[corrected: Bob]` and \\[corrected: Bob\\] [corrected: Bob](target)\n";
    assert_eq!(
        extract(literal).unwrap().annotate(&[None]).unwrap(),
        literal
    );
}

#[test]
fn annotation_payloads_are_escaped_and_repeated_checks_preserve_identity() {
    let source = "- Alice paid &#36;4 &amp; smiled [00:00-00:01]\n";
    let findings = [Some(Finding::Unsupported(
        "Bob] <!-- vpt:content end -->\n# secret".into(),
    ))];
    let parsed = extract(source).unwrap();
    assert_eq!(parsed.claims[0].claim.text, "Alice paid $4 & smiled");
    let rendered = parsed.annotate(&findings).unwrap();
    assert!(!rendered.contains("<!-- vpt:content end -->"));
    let again = extract(&rendered).unwrap();
    assert_eq!(
        again.claims[0].identity_text,
        parsed.claims[0].identity_text
    );
    assert_eq!(again.annotate(&findings).unwrap(), rendered);
}

#[test]
fn paragraph_annotations_stay_with_their_sentences_and_citations() {
    let source = "First sentence. [00:00-00:01] Second sentence? [00:02-00:03]\n";
    let parsed = extract(source).unwrap();
    let findings = [
        Some(Finding::Unsupported("First".into())),
        Some(Finding::BuiltOnFlaggedText),
    ];
    let rendered = parsed.annotate(&findings).unwrap();
    assert_eq!(
        rendered,
        "First sentence. [00:00-00:01] [unsupported: First] Second sentence? [00:02-00:03] [built-on-flagged-text]\n"
    );
    let again = extract(&rendered).unwrap();
    assert_eq!(again.claims.len(), 2);
    for (old, new) in parsed.claims.iter().zip(&again.claims) {
        assert_eq!(old.identity_text, new.identity_text);
    }
    assert_eq!(again.annotate(&findings).unwrap(), rendered);
}

#[test]
fn annotations_do_not_touch_excluded_blocks_or_link_labels() {
    let source =
        "# [unsourced]\n\n```\n[unsourced]\n```\n\n- [unsourced](target.md) [bad-reference]\n";
    let parsed = extract(source).unwrap();
    assert_eq!(parsed.claims.len(), 1);
    assert_eq!(parsed.claims[0].claim.text, "unsourced");
    assert_eq!(
        parsed.annotate(&[None]).unwrap(),
        "# [unsourced]\n\n```\n[unsourced]\n```\n\n- [unsourced](target.md)\n"
    );
}

#[test]
fn repeated_old_markers_are_removed_without_merging_adjacent_words() {
    let parsed = extract("- claim [unsourced] [bad-reference] continued\n").unwrap();
    assert_eq!(parsed.claims[0].claim.text, "claim continued");
    assert_eq!(
        parsed.annotate(&[Some(Finding::Unsourced)]).unwrap(),
        "- claim continued [unsourced]\n"
    );
}

#[test]
fn formatting_entities_and_quotes_keep_claim_identity_after_annotation() {
    for source in [
        "**First.** Second.\n",
        "&#34;First?&#34; Second!\n",
        "First&#46; Second.\n",
        "- a&#32;[unsourced] b [00:00-00:01]\n",
        "- a [unsourced]\n  continued [00:00-00:01]\n",
        "- **bold** [00:00-00:01]\n",
        "- [Alice](target.md)\n",
        "- [[Alice|Alicia]]\n",
    ] {
        let parsed = extract(source).unwrap();
        let findings = vec![Some(Finding::Unsourced); parsed.claims.len()];
        let rendered = parsed.annotate(&findings).unwrap();
        let again = extract(&rendered).unwrap();
        assert_eq!(
            parsed
                .claims
                .iter()
                .map(|claim| &claim.identity_text)
                .collect::<Vec<_>>(),
            again
                .claims
                .iter()
                .map(|claim| &claim.identity_text)
                .collect::<Vec<_>>(),
            "{source}"
        );
        assert_eq!(again.annotate(&findings).unwrap(), rendered, "{source}");
    }
}
