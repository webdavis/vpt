use super::*;
use vpt_domain::{
    digest::Sha256Digest,
    note::NoteStage,
    review::{FlagContent, Resolution, ReviewFlag, VerificationFlag},
    time::UtcInstant,
};

fn finding(source: &str) -> ReviewFlag {
    let parsed = extract_owned(source, NoteStage::Transcript, &[]).unwrap();
    ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution: Resolution::Corrected {
            text: "Alice".into(),
            at: UtcInstant { secs: 1 },
        },
        content: FlagContent::Verification(VerificationFlag {
            artifact: NoteStage::Transcript,
            claim_index: 0,
            claim_digest: Sha256Digest([2; 32]),
            record_text: parsed.claims()[0].identity_text.clone(),
            finding: Finding::Unsourced,
            sources: vec![],
        }),
    }
}

#[test]
fn edited_claims_lose_finding_annotations_while_retaining_identical_lexical_corrections() {
    let source = "[00:00] met [corrected: Alice]\n";
    let flag = finding(source);
    let parsed = extract_owned(source, NoteStage::Transcript, std::slice::from_ref(&flag)).unwrap();
    let rendered = parsed
        .annotate_resolved(
            &[Some(Finding::Unsourced)],
            std::slice::from_ref(&flag.resolution),
        )
        .unwrap();
    assert_eq!(rendered.matches("[corrected: Alice]").count(), 2);
    let changed = rendered.replace("met", "called");
    let parsed = extract_owned(&changed, NoteStage::Transcript, &[flag]).unwrap();
    assert_eq!(
        parsed.annotate(&[None]).unwrap(),
        source.replace("met", "called")
    );
    assert!(
        parsed.claims()[0]
            .identity_text
            .contains("[corrected: Alice]")
    );
}

#[test]
fn lexical_correction_punctuation_and_references_remain_claim_text() {
    let source = "[00:00] [corrected: Alice. Bob? &#91;00:02-00:03&#93;] [00:00-00:01]\n";
    let parsed = extract_owned(source, NoteStage::Transcript, &[]).unwrap();
    assert_eq!(parsed.claims().len(), 1);
    assert_eq!(parsed.claims()[0].claim.source_references, ["00:00-00:01"]);
    assert!(parsed.claims()[0].claim.text.contains("Alice. Bob?"));
    assert_eq!(parsed.annotate(&[None]).unwrap(), source);
}

#[test]
fn saved_legacy_corrections_are_upgraded_without_treating_lexical_markers_as_annotations() {
    let source = "[00:00] met [corrected: Alice]\n";
    let flag = finding(source);
    let legacy = "[00:00] met [corrected: Alice] [corrected: Alice]\n";
    let parsed = extract_owned(legacy, NoteStage::Transcript, std::slice::from_ref(&flag)).unwrap();
    assert_eq!(parsed.annotate(&[None]).unwrap(), source);
    let upgraded = parsed
        .annotate_resolved(
            &[Some(Finding::Unsourced)],
            std::slice::from_ref(&flag.resolution),
        )
        .unwrap();
    assert_eq!(
        upgraded
            .matches(vpt_domain::note::FINDING_CORRECTION_MARKER)
            .count(),
        1
    );
    let again = extract_owned(&upgraded, NoteStage::Transcript, &[]).unwrap();
    assert_eq!(again.annotate(&[None]).unwrap(), source);
}

#[test]
fn marked_corrections_round_trip_across_formatted_and_adjacent_claims() {
    for source in [
        "**First.** Second?\n",
        "- **bold** [00:00-00:01]\n",
        "- first\n- second\n",
        "First. [00:00-00:01] Second! [00:01-00:02]\n",
    ] {
        let parsed = extract_owned(source, NoteStage::Transcript, &[]).unwrap();
        let count = parsed.claims().len();
        let resolutions = vec![
            Resolution::Corrected {
                text: "Alice. &#93; Bob?\nNext".into(),
                at: UtcInstant { secs: 1 }
            };
            count
        ];
        let annotations = vec![Some(Finding::Unsourced); count];
        let rendered = parsed
            .annotate_resolved(&annotations, &resolutions)
            .unwrap();
        let again = extract_owned(&rendered, NoteStage::Transcript, &[]).unwrap();
        assert_eq!(again.annotate(&vec![None; count]).unwrap(), source);
        assert_eq!(
            again.annotate_resolved(&annotations, &resolutions).unwrap(),
            rendered
        );
    }
}

#[test]
fn fingerprints_track_claims_and_citations_but_ignore_verification_and_headings() {
    let source = "# Heading\n\n- met [corrected: Alice] [00:00-00:01]\n";
    let parsed = extract_owned(source, NoteStage::Transcript, &[]).unwrap();
    let annotated = parsed
        .annotate_resolved(
            &[Some(Finding::Unsourced)],
            &[Resolution::Corrected {
                text: "Bob".into(),
                at: UtcInstant { secs: 1 },
            }],
        )
        .unwrap()
        .replace("# Heading", "# Renamed heading");
    assert_eq!(
        extract_owned(&annotated, NoteStage::Transcript, &[])
            .unwrap()
            .digest(),
        parsed.digest()
    );
    for changed in [
        source.replace("Alice", "Bob"),
        source.replace("00:00-00:01", "00:01-00:02"),
        format!("{source}- another claim\n"),
    ] {
        assert_ne!(
            extract_owned(&changed, NoteStage::Transcript, &[])
                .unwrap()
                .digest(),
            parsed.digest()
        );
    }
}
