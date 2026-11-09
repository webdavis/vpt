use super::*;
use vpt_domain::{grounding::Finding, note::NoteStage, review::VerificationFlag};

#[test]
fn finding_only_review_counts_each_class_without_retaining_claims() {
    let mut review = review();
    for (index, finding) in [
        Finding::Unsourced,
        Finding::BadReference,
        Finding::Unsupported("private token".into()),
        Finding::BuiltOnFlaggedText,
    ]
    .into_iter()
    .enumerate()
    {
        review.findings.push((
            Sha256Digest([index as u8; 32]),
            VerificationFlag {
                artifact: NoteStage::Analysis,
                claim_index: index as u32,
                claim_digest: Sha256Digest([index as u8; 32]),
                record_text: "private claim".into(),
                finding,
                sources: vec![],
            },
        ));
    }
    let notice = ReviewNotice::from_review(&review, "/notes/analysis.md".into()).unwrap();
    assert_eq!(notice.counts.unsourced, 1);
    assert_eq!(notice.counts.bad_reference, 1);
    assert_eq!(notice.counts.unsupported, 1);
    assert_eq!(notice.counts.built_on_flagged_text, 1);
    assert_eq!(
        notice.event(UtcInstant { secs: 20 }).detail,
        EventDetail::Review {
            spans: 4,
            agreed_unverified: 0
        }
    );
    assert!(!format!("{notice:?}").contains("private"));
}

#[test]
fn final_notice_replaces_prior_counts_and_failed_replacement_preserves_it() {
    let mut review = review();
    review
        .diagnostics
        .push((Sha256Digest([1; 32]), Diagnostic::SingleEngine));
    let mut notice = ReviewNotice::from_review(&review, "/notes/transcript.md".into()).unwrap();
    let mut queue = ReviewQueue::default();
    queue.push(notice.clone()).unwrap();
    notice.counts.unsourced = 1;
    notice.path = "/notes/analysis.md".into();
    queue.replace(notice.clone()).unwrap();
    let at = UtcInstant { secs: 20 };
    let expected = vec![notice.event(at)];
    assert_eq!(queue.events(3, at).unwrap(), expected);
    notice.path = "x".repeat(16 * 1024 * 1024 + 1).into();
    assert_eq!(queue.replace(notice), Err(AlertError::CapacityExceeded));
    assert_eq!(queue.events(3, at).unwrap(), expected);
    queue.remove(&review.recording);
    assert!(queue.events(3, at).unwrap().is_empty());
    queue
        .push(ReviewNotice::from_review(&review, "/notes/new.md".into()).unwrap())
        .unwrap();
    assert_eq!(queue.events(3, at).unwrap().len(), 1);
}
