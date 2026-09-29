use super::*;
use crate::ledger::sqlite::history::retention::tests::application_calls::{Calls, PostCommit};
use vpt_application::{RetentionCommit, reconcile_intents};

pub(in crate::ledger::sqlite::history::retention::tests) fn preparation(
    ledger: &SqliteLedger,
    root: &RootDir,
) {
    let fixture = Fixture::new(ledger, root);
    let mut counted = Calls::new(ledger);
    counted.post_commit = PostCommit::Preparation;
    let mut sink = Sink::default();
    let failed = fixture.run(&counted, &mut sink);
    assert_eq!(
        failed.error,
        RetentionError::RequiredHistory(LedgerError::Io("owned post-commit observation".into()))
    );
    let Some(RetentionCommit::Preparation(prepared)) = failed.committed else {
        panic!("known preparation receipt was lost");
    };
    assert!(prepared.intent_id > 0);
    assert_eq!(prepared.effect_id, prepared.receipt.records[0].id);
    assert_eq!(ledger.pending_intents().unwrap()[0].id, prepared.intent_id);
    assert_eq!(
        ledger.recovery_view().unwrap().records.last(),
        prepared.receipt.records.last()
    );
    assert!(failed.report.moved.is_empty());
    assert!(failed.report.completed.is_empty());
    assert_eq!(fixture.trash.calls.get(), 0);
    assert_eq!(counted.writes.get(), 2);
    assert_eq!(sink.receipts.len(), 1);
    assert_eq!(
        std::fs::read(fixture.record.audio_path).unwrap(),
        b"owned audio for failure proof"
    );
}

pub(in crate::ledger::sqlite::history::retention::tests) fn completion(
    ledger: &SqliteLedger,
    root: &RootDir,
) {
    let fixture = Fixture::new(ledger, root);
    let mut counted = Calls::new(ledger);
    counted.post_commit = PostCommit::Completion;
    let mut sink = Sink::default();
    let failed = fixture.run(&counted, &mut sink);
    assert_eq!(
        failed.error,
        RetentionError::RequiredHistory(LedgerError::Io("owned post-commit observation".into()))
    );
    let Some(RetentionCommit::Step(receipt)) = failed.committed else {
        panic!("known completion receipt was lost");
    };
    assert_eq!(receipt.completed, vec![fixture.record.id.clone()]);
    assert_eq!(receipt.records.len(), 2);
    assert_eq!(failed.report.completed, receipt.completed);
    assert_eq!(failed.report.moved.len(), 1);
    assert_eq!(failed.report.moved[0].recording, fixture.record.id);
    assert_eq!(fixture.trash.calls.get(), 1);
    assert_eq!(counted.writes.get(), 3);
    assert_eq!(sink.receipts.len(), 2);
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        &ledger.recovery_view().unwrap().records[2..],
        receipt.records.as_slice()
    );
    assert_eq!(
        std::fs::read(fixture.trash.directory.join("owned.m4a")).unwrap(),
        b"owned audio for failure proof"
    );
}

pub(in crate::ledger::sqlite::history::retention::tests) fn absent(
    ledger: &SqliteLedger,
    root: &RootDir,
) {
    let fixture = Fixture::new(ledger, root);
    let run = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 120 })).run;
    let prepared = committed(ledger.prepare_retention(
        run,
        &NewIntent {
            kind: ArtifactKind::Audio,
            recording: fixture.record.id.clone(),
            path: fixture.record.audio_path.clone(),
            expected: fixture.record.digest,
            recorded_at: UtcInstant { secs: 121 },
        },
    ));
    std::fs::rename(
        &fixture.record.audio_path,
        fixture.trash.directory.join("already-moved.m4a"),
    )
    .unwrap();
    let recovered = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 122 })).run;
    let mut counted = Calls::new(ledger);
    counted.post_commit = PostCommit::Completion;
    let mut sink = Sink::default();
    let failed = reconcile_intents(
        recovered,
        &counted,
        &fixture.stores,
        &fixture.trash,
        &FixedClock,
        &fixture.paths.audio,
        &mut sink,
    )
    .unwrap_err();
    assert_eq!(
        failed.error,
        RetentionError::RequiredHistory(LedgerError::Io("owned post-commit observation".into()))
    );
    let Some(RetentionCommit::Step(receipt)) = failed.committed else {
        panic!("known absent-target completion receipt was lost");
    };
    assert_eq!(receipt.run, recovered);
    assert_eq!(receipt.completed, vec![fixture.record.id.clone()]);
    assert_eq!(
        receipt.records[0].fact,
        HistoryFact::EffectObserved {
            intent: prepared.effect_id,
            outcome: EffectOutcome::Completed
        }
    );
    assert_eq!(failed.progress.completed, receipt.completed);
    assert!(failed.progress.moved.is_empty());
    assert_eq!(fixture.trash.calls.get(), 0);
    assert_eq!(counted.writes.get(), 1);
    assert!(sink.receipts.is_empty());
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        std::fs::read(fixture.trash.directory.join("already-moved.m4a")).unwrap(),
        b"owned audio for failure proof"
    );
}
