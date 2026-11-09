use super::*;
use vpt_domain::{
    digest::Sha256Digest,
    grounding::{Finding, SourceRange},
    identity::RecordingId,
    note::NoteStage,
    review::VerificationFlag,
};

#[test]
fn review_reports_claim_identity_and_null_sources_without_inventing_word_offsets() {
    let report = Review {
        recording: RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap(),
        transcribed: true,
        flags: vec![],
        diagnostics: vec![],
        findings: vec![(
            Sha256Digest([1; 32]),
            VerificationFlag {
                artifact: NoteStage::Analysis,
                claim_index: 3,
                claim_digest: Sha256Digest([2; 32]),
                record_text: "private claim\nwith control formatting".into(),
                finding: Finding::Unsourced,
                sources: vec![],
            },
        )],
    };
    let Outcome::Success {
        document, human, ..
    } = success(&report)
    else {
        panic!("expected successful review");
    };
    assert_eq!(document["open_flags"], 1);
    assert_eq!(document["findings"][0]["class"], "unsourced");
    assert_eq!(document["findings"][0]["artifact"], "analysis");
    assert_eq!(document["findings"][0]["claim_index"], 3);
    assert!(document["findings"][0]["sources"].is_null());
    assert_eq!(
        document["findings"][0]["claim_digest"],
        Sha256Digest([2; 32]).hex()
    );
    assert!(document["findings"][0].get("word_range").is_none());
    assert!(human.contains("1 open verification findings"));
    assert!(human.contains("private claim\\nwith control formatting"));
}

#[test]
fn sourced_findings_report_time_ranges_and_the_unsupported_token() {
    let finding = VerificationFlag {
        artifact: NoteStage::Transcript,
        claim_index: 0,
        claim_digest: Sha256Digest([2; 32]),
        record_text: "Private claim [00:03-00:07]".into(),
        finding: Finding::Unsupported("Private".into()),
        sources: vec![SourceRange {
            start_secs: 3,
            end_secs: 7,
        }],
    };
    let value = finding_json(&Sha256Digest([1; 32]), &finding);
    assert_eq!(value["sources"], json!([{"start":3,"end":7}]));
    assert_eq!(value["unsupported_token"], "Private");
}
