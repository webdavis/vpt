use super::*;

pub(crate) fn acceptance_invalidates_note_success(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let mut row = record(81);
    row.stages.note = StageState::Succeeded;
    row.stages.analysis = StageState::Succeeded;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Transcribe, UtcInstant { secs: 49 })).run;
    committed(ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]));
    let after = ledger.by_id(&row.id).unwrap().unwrap();
    assert_eq!(after.stages.note, StageState::Pending);
    assert_eq!(after.stages.analysis, StageState::Succeeded);
}

pub(crate) fn acceptance_preserves_expired_notes(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let mut row = record(81);
    row.stages.note = StageState::Expired;
    row.stages.analysis = StageState::Expired;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Transcribe, UtcInstant { secs: 49 })).run;
    committed(ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]));
    let after = ledger.by_id(&row.id).unwrap().unwrap();
    assert_eq!(after.stages.note, StageState::Expired);
    assert_eq!(after.stages.analysis, StageState::Expired);
}

pub(crate) fn rejected_acceptance_preserves_note_success_and_prior_state(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let mut row = record(81);
    row.stages.note = StageState::Succeeded;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Transcribe, UtcInstant { secs: 49 })).run;
    let facts = [
        fact(WorkOutcome::Completed),
        (
            UtcInstant { secs: 50 },
            HistoryFact::EffectObserved {
                intent: vpt_domain::history::EventId::new(run.namespace(), 9999),
                outcome: vpt_domain::run::EffectOutcome::Completed,
            },
        ),
    ];
    assert!(matches!(
        ledger.commit_step(run, &delta(), &facts),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.by_id(&row.id).unwrap(), Some(row));
    assert!(
        ledger
            .accepted_transcript(&fixture().recording)
            .unwrap()
            .is_none()
    );
}
