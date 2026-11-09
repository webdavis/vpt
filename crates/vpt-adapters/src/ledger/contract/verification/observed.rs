use super::*;

pub(crate) fn observed_claim_edits_commit_only_with_the_checked_source_and_matching_publication(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let mut change = change(ledger);
    let old = ledger
        .note(&change.recording, change.artifact)
        .unwrap()
        .unwrap();
    let mut observed = old.clone();
    fixture::replace_content(&mut observed, "- changed claim\n");
    change.observed = Some(StoredNote {
        text: observed.text.clone(),
        digest: observed.digest,
    });
    change.input_digest = observed.content_digest;
    change.findings[0].record_text = "changed claim".into();
    change.findings[0].claim_digest = notes::hash(b"changed claim");
    let mut next = observed.clone();
    fixture::replace_content(&mut next, "- changed claim [unsourced]\n");
    next.updated_at = change.at;
    let delta = WorkDelta {
        verification: Some(change),
        notes: vec![NoteChange {
            artifact: next.clone(),
            expected_saved: Some(old.digest),
            relocated_from: None,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: old.target.clone(),
                expected_previous: Some(observed.digest),
                intended: next.digest,
                recorded_at: next.updated_at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let history = ledger.pending_exports().unwrap();
    for variant in 0..4 {
        let mut bad = delta.clone();
        match variant {
            0 => bad.verification.as_mut().unwrap().input_digest = old.content_digest,
            1 => {
                bad.verification
                    .as_mut()
                    .unwrap()
                    .observed
                    .as_mut()
                    .unwrap()
                    .digest = old.digest
            }
            2 => bad.commit.publications[0].expected_previous = Some(old.digest),
            3 => {
                fixture::replace_content(&mut bad.notes[0].artifact, "different unchecked claim\n");
                bad.commit.publications[0].intended = bad.notes[0].artifact.digest;
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                ledger.commit_step(run, &bad, &[fixture::fact(80, WorkOutcome::Completed)]),
                HistoryWrite::Unconfirmed(_)
            ),
            "variant {variant}"
        );
        assert_eq!(
            ledger.note(&old.recording, old.stage).unwrap(),
            Some(old.clone())
        );
        assert!(findings(ledger).is_empty());
        assert_eq!(ledger.pending_exports().unwrap(), history);
    }
    committed(ledger.commit_step(run, &delta, &[fixture::fact(80, WorkOutcome::Completed)]));
    assert_eq!(ledger.note(&old.recording, old.stage).unwrap(), Some(next));
    assert_eq!(findings(ledger).len(), 1);
    assert_eq!(
        ledger.pending_publications().unwrap(),
        delta.commit.publications
    );
}
