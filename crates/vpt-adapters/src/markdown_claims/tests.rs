use super::*;
mod corrections;
use vpt_domain::grounding::Finding;

#[test]
fn extracts_list_items_and_paragraph_sentences_excluding_document_structure() {
    let source = "---\ntitle: Secret Header\n---\n# Heading\n\n<!-- vpt:links start -->\n- [Private link](private-target.md)\n<!-- vpt:links end -->\n\n```text\nHidden Code.\n```\n\n- Alice met Bob. They agreed. [00:00-00:03]\n- another action\n\nThe cost is $1.25. [00:04-00:08] Next sentence!\n";
    let result = extract(source).unwrap();
    assert_eq!(result.claims.len(), 4);
    assert_eq!(result.claims[0].claim.text, "Alice met Bob. They agreed.");
    assert_eq!(result.claims[0].claim.source_references, ["00:00-00:03"]);
    assert_eq!(result.claims[1].claim.text, "another action");
    assert_eq!(result.claims[2].claim.text, "The cost is $1.25.");
    assert_eq!(result.claims[2].claim.source_references, ["00:04-00:08"]);
    assert_eq!(result.claims[3].claim.text, "Next sentence!");
    assert_eq!(
        result
            .claims
            .iter()
            .map(|claim| claim.index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
}

#[test]
fn annotations_are_replaced_without_changing_claim_identity_or_surrounding_bytes() {
    let source = "# Private heading\n\n- Alice [00:00-00:02] [unsupported: Bob]\n- unsourced action [unsourced]\n";
    let parsed = extract(source).unwrap();
    assert_eq!(parsed.claims.len(), 2);
    let updated = parsed
        .annotate(&[Some(Finding::BuiltOnFlaggedText), None])
        .unwrap();
    assert_eq!(
        updated,
        "# Private heading\n\n- Alice [00:00-00:02] [built-on-flagged-text]\n- unsourced action\n"
    );
    let again = extract(&updated).unwrap();
    assert_eq!(
        again
            .claims
            .iter()
            .map(|claim| &claim.identity_text)
            .collect::<Vec<_>>(),
        parsed
            .claims
            .iter()
            .map(|claim| &claim.identity_text)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        again
            .annotate(&[Some(Finding::BuiltOnFlaggedText), None])
            .unwrap(),
        updated
    );
    assert!(!format!("{:?}", parsed.claims).contains("Alice"));
}

mod annotations;
mod limits;
mod structure;

mod grounding;
mod locations;
mod resolutions;
