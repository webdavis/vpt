use super::*;
use vpt_domain::{review::Resolution, time::UtcInstant};

#[test]
fn reviewed_annotations_round_trip_without_changing_claims_or_literal_markup() {
    let source = "- open [unsourced]\n- confirmed [bad-reference]\n- dismissed [unsupported: Private]\n- original [00:00-00:01] [built-on-flagged-text]\n";
    let at = UtcInstant { secs: 10 };
    let resolutions = [
        Resolution::Open,
        Resolution::Confirmed { at },
        Resolution::Dismissed { at },
        Resolution::Corrected {
            text: "Private]\n[00:10-00:20]".into(),
            at,
        },
    ];
    let findings = vec![Some(Finding::Unsourced); 4];
    let parsed = extract(source).unwrap();
    let rendered = parsed.annotate_resolved(&findings, &resolutions).unwrap();
    assert_eq!(
        rendered,
        "- open [unsourced]\n- confirmed\n- dismissed\n- original [00:00-00:01] [corrected: Private&#93;&#10;&#91;00&#58;10-00&#58;20&#93;]<!-- vpt:verification-correction -->\n"
    );
    let again = extract(&rendered).unwrap();
    assert_eq!(
        parsed
            .claims
            .iter()
            .map(|c| &c.identity_text)
            .collect::<Vec<_>>(),
        again
            .claims
            .iter()
            .map(|c| &c.identity_text)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        again.annotate_resolved(&findings, &resolutions).unwrap(),
        rendered
    );
    assert_eq!(again.claims[3].claim.source_references, ["00:00-00:01"]);
}

#[test]
fn resolution_count_must_match_claim_count() {
    let parsed = extract("- claim").unwrap();
    assert_eq!(
        parsed.annotate_resolved(&[Some(Finding::Unsourced)], &[]),
        Err(ClaimError::InvalidAnnotations)
    );
}

#[test]
fn correction_payloads_share_the_total_annotation_budget() {
    let source = "- claim\n".repeat(257);
    let parsed = extract(&source).unwrap();
    let resolutions = vec![
        Resolution::Corrected {
            text: "x".repeat(65_536),
            at: UtcInstant { secs: 1 }
        };
        257
    ];
    assert_eq!(
        parsed.annotate_resolved(&vec![Some(Finding::Unsourced); 257], &resolutions),
        Err(ClaimError::CapacityExceeded)
    );
}
