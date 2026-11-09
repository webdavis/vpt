use super::*;

fn rewrite(ledger: &impl proposals::Ledger, run: vpt_domain::history::RunId) {
    let old = ledger
        .note(&record(81).id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let mut next = old.clone();
    let content = crate::note_document::Document::parse(&old.text)
        .unwrap()
        .content()
        .unwrap()
        .to_owned()
        + "\n- additional claim\n";
    fixture::replace_content(&mut next, &content);
    next.updated_at = UtcInstant { secs: 90 };
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            notes: vec![NoteChange {
                artifact: next.clone(),
                expected_saved: Some(old.digest),
                relocated_from: None,
            }],
            commit: LedgerCommit {
                publications: vec![DirtyPublication {
                    target: next.target,
                    expected_previous: Some(old.digest),
                    intended: next.digest,
                    recorded_at: next.updated_at,
                }],
                ..Default::default()
            },
            ..Default::default()
        },
        &[fixture::fact(90, WorkOutcome::Completed)],
    ));
}

pub(crate) fn changed_owned_claims_invalidate_only_the_changed_artifact(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    analysis(ledger, run);
    committed(save(ledger, run, change(ledger)));
    rewrite(ledger, run);
    let review = ledger.review_snapshot(&record(81).id).unwrap().unwrap();
    assert_eq!(review.stale_artifacts, vec![NoteStage::Transcript]);
    assert_eq!(review.stale_findings.len(), 1);
    assert!(findings(ledger).iter().all(|flag| matches!(&flag.content,vpt_domain::review::FlagContent::Verification(finding) if finding.artifact == NoteStage::Analysis)));
}

pub(crate) fn changed_claims_invalidate_an_empty_verification_result(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let mut change = change(ledger);
    change.findings.clear();
    committed(save(ledger, run, change));
    rewrite(ledger, run);
    let review = ledger.note_inputs(&record(81).id).unwrap().unwrap().review;
    assert_eq!(review.stale_artifacts, vec![NoteStage::Transcript]);
    assert!(review.stale_findings.is_empty());
}
