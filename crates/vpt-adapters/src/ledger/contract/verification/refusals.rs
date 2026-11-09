use super::*;
use vpt_domain::{grounding::SourceRange, run::HistoryFact};

pub(crate) fn invalid_or_stale_findings_leave_state_and_history_unchanged(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let good = change(ledger);
    committed(save(ledger, run, good.clone()));
    let before = findings(ledger);
    let history = ledger.pending_exports().unwrap();
    for variant in 0..12 {
        let mut change = good.clone();
        match variant {
            0 => change.input_digest = super::super::digest(99),
            1 => change.basis.transcript = super::super::digest(99),
            2 => change.basis.lexical_flags = super::super::digest(99),
            3 => change.findings[0].claim_digest = super::super::digest(99),
            4 => change.findings[0].artifact = NoteStage::Analysis,
            5 => change.findings.push(change.findings[0].clone()),
            6 => change.findings[0].claim_index = 65_536,
            7 => change.findings[0].sources.push(SourceRange {
                start_secs: 0,
                end_secs: 1,
            }),
            8 => {
                change.findings[0].finding = Finding::Unsupported("private invalid token".into());
            }
            9 => {
                change.findings[0].finding = Finding::BuiltOnFlaggedText;
                change.findings[0].sources.push(SourceRange {
                    start_secs: 0,
                    end_secs: 3,
                });
            }
            10 => change.at = UtcInstant { secs: 79 },
            11 => change.artifact = NoteStage::Analysis,
            _ => unreachable!(),
        }
        assert!(
            matches!(save(ledger, run, change), HistoryWrite::Unconfirmed(_)),
            "variant {variant}"
        );
        assert_eq!(findings(ledger), before);
        assert_eq!(ledger.pending_exports().unwrap(), history);
    }
}

pub(crate) fn late_history_failure_rolls_back_finding_replacement(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let mut change = change(ledger);
    committed(save(ledger, run, change.clone()));
    let before = findings(ledger);
    let history = ledger.pending_exports().unwrap();
    change.findings.clear();
    let result = ledger.commit_step(
        run,
        &WorkDelta {
            verification: Some(change),
            ..Default::default()
        },
        &[
            fixture::fact(80, WorkOutcome::Completed),
            (
                UtcInstant { secs: 80 },
                HistoryFact::HookFailed {
                    event: vpt_domain::history::EventId::new(run.namespace(), 999_999),
                    failure: vpt_domain::run::HookFailureClass::Spawn,
                },
            ),
        ],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)));
    assert_eq!(findings(ledger), before);
    assert_eq!(ledger.pending_exports().unwrap(), history);
}

pub(crate) fn pending_publication_and_retention_refuse_verification(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let change = change(ledger);
    let note = ledger
        .note(&change.recording, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    ledger
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: note.target.clone(),
                expected_previous: Some(note.digest),
                intended: note.digest,
                recorded_at: change.at,
            }],
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        save(ledger, run, change.clone()),
        HistoryWrite::Unconfirmed(_)
    ));
    ledger.clear_publication(&note.target).unwrap();
    ledger
        .record_intent(&NewIntent {
            kind: ArtifactKind::Audio,
            owner: (change.recording.clone()).into(),
            path: note.target,
            expected: note.digest,
            recorded_at: change.at,
        })
        .unwrap();
    assert!(matches!(
        save(ledger, run, change),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(findings(ledger).is_empty());
}

pub(crate) fn same_batch_content_change_refuses_stale_findings(
    ledger: &(impl proposals::Ledger + ReviewLedger),
) {
    let run = fixture::seed(ledger);
    let change = change(ledger);
    let old = ledger
        .note(&change.recording, change.artifact)
        .unwrap()
        .unwrap();
    let mut next = old.clone();
    fixture::replace_content(&mut next, "different claim\n");
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
                target: next.target.clone(),
                expected_previous: Some(old.digest),
                intended: next.digest,
                recorded_at: next.updated_at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let history = ledger.pending_exports().unwrap();
    assert!(matches!(
        ledger.commit_step(run, &delta, &[fixture::fact(80, WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.note(&old.recording, old.stage).unwrap(), Some(old));
    assert!(findings(ledger).is_empty());
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.pending_exports().unwrap(), history);
}
