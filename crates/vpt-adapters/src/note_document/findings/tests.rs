use super::*;
use vpt_domain::{grounding::Finding, review::VerificationFlag, time::UtcInstant};

fn flag(content: &str, stage: NoteStage, resolution: Resolution) -> ReviewFlag {
    let parsed = markdown_claims::extract_content(content).unwrap();
    let claim = &parsed.claims()[0];
    ReviewFlag {
        identifier: Sha256Digest([1; 32]),
        resolution,
        content: FlagContent::Verification(VerificationFlag {
            artifact: stage,
            claim_index: 0,
            claim_digest: Sha256Digest(Sha256::digest(claim.identity_text.as_bytes()).into()),
            record_text: claim.identity_text.clone(),
            finding: Finding::Unsourced,
            sources: vec![],
        }),
    }
}

#[test]
fn saved_resolution_is_scoped_to_the_stage_position_and_exact_claim() {
    let source = "## Summary\n\n- a claim [unsourced]\n";
    let resolution = Resolution::Corrected {
        text: "operator] correction".into(),
        at: UtcInstant { secs: 20 },
    };
    let saved = flag(source, NoteStage::Analysis, resolution);
    assert_eq!(
        render(source, NoteStage::Analysis, std::slice::from_ref(&saved)).unwrap(),
        "## Summary\n\n- a claim [corrected: operator&#93; correction]<!-- vpt:verification-correction -->\n"
    );
    assert_eq!(
        render(source, NoteStage::Transcript, std::slice::from_ref(&saved)).unwrap(),
        source
    );
    let changed = source.replace("a claim", "another claim");
    assert_eq!(
        render(&changed, NoteStage::Analysis, std::slice::from_ref(&saved)).unwrap(),
        "## Summary\n\n- another claim\n"
    );
    let moved = format!("- earlier claim\n{source}");
    assert!(
        !render(&moved, NoteStage::Analysis, &[saved])
            .unwrap()
            .contains("corrected")
    );
}

#[test]
fn confirmed_and_dismissed_findings_remove_only_verification_annotations() {
    let source = "## Transcript\n\n[00:00] [unverified: word] [unsourced]\n";
    for resolution in [
        Resolution::Confirmed {
            at: UtcInstant { secs: 20 },
        },
        Resolution::Dismissed {
            at: UtcInstant { secs: 20 },
        },
    ] {
        let saved = flag(source, NoteStage::Transcript, resolution);
        assert_eq!(
            render(source, NoteStage::Transcript, &[saved]).unwrap(),
            "## Transcript\n\n[00:00] [unverified: word]\n"
        );
    }
}
