use super::{committed, record};
use std::path::PathBuf;
use vpt_application::ports::{HistoryWrite, LedgerCommit, RecordingLedger, RunHistory, WorkDelta};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{HistoryFact, RecoveryEvidence, RunKind};
use vpt_domain::time::UtcInstant;

fn invalid_evidence(run: RunId) -> RecoveryEvidence {
    RecoveryEvidence {
        affected_runs: vec![RunId::new(run.namespace(), 0)],
        first_failure_observed_at: None,
        logging_restored_observed_at: None,
        failures: 0,
        recovered_records: 0,
        completed_work: 0,
        failed_work: 0,
        unknown_work: 0,
        deferred_work: 0,
        completed_effects: 0,
        failed_effects: 0,
        unresolved_effects: 0,
        missing_history: 0,
    }
}

pub(crate) fn zero_affected_run_atomic_begin_has_no_partial_history<L: RunHistory>(ledger: &L) {
    let original = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let before = ledger.recovery_view().unwrap();
    let refused = ledger.begin_recovery(UtcInstant { secs: 11 }, invalid_evidence(original.run));
    assert!(
        matches!(refused, HistoryWrite::Unconfirmed(_)),
        "{refused:?}"
    );
    assert_eq!(ledger.recovery_view().unwrap(), before);

    let accepted = committed(ledger.begin_recovery(
        UtcInstant { secs: 12 },
        RecoveryEvidence {
            affected_runs: vec![original.run],
            ..invalid_evidence(original.run)
        },
    ));
    assert_eq!(accepted.run.sequence(), original.run.sequence() + 1);
    assert_eq!(accepted.records.len(), 2);
    assert_eq!(accepted.records[0].run, accepted.run);
    assert_eq!(accepted.records[1].run, accepted.run);
    assert_eq!(
        accepted.records[0].fact,
        HistoryFact::RunStarted(RunKind::Recover)
    );
    assert!(matches!(
        accepted.records[1].fact,
        HistoryFact::RecoveryProgress { .. }
    ));
    let after = ledger.recovery_view().unwrap();
    assert_eq!(
        &after.records[..before.records.len()],
        before.records.as_slice()
    );
    assert_eq!(
        &after.records[before.records.len()..],
        accepted.records.as_slice()
    );
}

fn reject_with_atomic_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
    fact: impl FnOnce(RunId, EventId) -> HistoryFact,
) {
    let recording = record(72);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let failure = ledger.commit_step(
        started.run,
        &WorkDelta {
            source_updates: vec![(
                recording.id.clone(),
                PathBuf::from("/vm/Recordings/should-not-appear.m4a"),
            )],
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 11 },
            fact(started.run, started.records[0].id),
        )],
    );
    assert!(
        matches!(failure, HistoryWrite::Unconfirmed(_)),
        "{failure:?}"
    );
    assert_eq!(ledger.by_id(&recording.id).unwrap(), Some(recording));
    assert_eq!(ledger.pending_exports().unwrap(), started.records);
}

pub(crate) fn zero_affected_run_progress_rolls_back_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    reject_with_atomic_work(ledger, |run, first| HistoryFact::RecoveryProgress {
        evidence: invalid_evidence(run),
        baseline: vpt_domain::run::RecoveryBaseline {
            generation: None,
            pending_from: Some(first),
            through: Some(first),
        },
    });
}

pub(crate) fn zero_affected_run_windowless_summary_rolls_back_work<
    L: RunHistory + RecordingLedger,
>(
    ledger: &L,
) {
    reject_with_atomic_work(ledger, |run, _| HistoryFact::RecoverySummary {
        evidence: invalid_evidence(run),
        window: None,
    });
}

pub(crate) fn positive_missing_retention_id_is_saved<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let fact = HistoryFact::MissingRetentionHistory { intent_id: 71 };
    let saved = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 11 }, fact.clone())],
    ));
    assert_eq!(saved.records[0].fact, fact);
    assert_eq!(
        ledger.recovery_view().unwrap().records.last(),
        saved.records.last()
    );
    assert_eq!(
        ledger.pending_exports().unwrap().last(),
        saved.records.last()
    );
}

pub(crate) fn zero_missing_retention_id_rolls_back_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    reject_with_atomic_work(ledger, |_, _| HistoryFact::MissingRetentionHistory {
        intent_id: 0,
    });
}
