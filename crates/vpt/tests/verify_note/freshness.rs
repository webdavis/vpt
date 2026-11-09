use super::*;

#[test]
fn readability_changes_invalidate_findings_before_counts_and_notes_are_published() {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let source = note::saved(&sandbox, &record);
    success(&invoke(&sandbox, &source.target, record.id.as_str()));
    let before = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap();
    assert_eq!(before.flags.len(), 1);
    fixture::write_config(&sandbox, "[readability]\nenabled = true\n");
    success(&note::invoke(&sandbox, record.id.as_str()));
    let current = note::saved(&sandbox, &record);
    assert!(!current.text.contains("[unsourced]"));
    assert!(
        current.text.contains("vptOpenFlags: 0"),
        "counts must omit findings checked against old claims"
    );
    let review = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap();
    assert!(review.flags.is_empty());
    assert_eq!(review.stale_findings, before.flags);
    assert_eq!(review.stale_artifacts, vec![NoteStage::Transcript]);
    success(&invoke(&sandbox, &source.target, record.id.as_str()));
    let checked = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap();
    assert_eq!(checked.flags.len(), 1);
    assert_ne!(checked.flags[0].identifier, before.flags[0].identifier);
    assert!(checked.stale_artifacts.is_empty());
}
