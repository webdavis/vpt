use super::{committed, record};
use std::path::PathBuf;
use vpt_application::ports::{HistoryWrite, LedgerCommit, RecordingLedger, RunHistory, WorkDelta};
use vpt_domain::history::RunId;
use vpt_domain::run::{HistoryFact, RecoveryEvidence, RunKind};
use vpt_domain::time::UtcInstant;

fn invalid_evidence(run: RunId) -> RecoveryEvidence {
    RecoveryEvidence {
        affected_runs: vec![RunId::new(run.namespace(), 0)],
        first_failure_observed_at: None,
        logging_restored_observed_at: None,
        failures: 0,
        recovered_records: 0,
        completed: 0,
        failed: 0,
        unresolved: 0,
        missing_history: 0,
    }
}

fn reject_with_atomic_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
    fact: impl FnOnce(RunId) -> HistoryFact,
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
        &[(UtcInstant { secs: 11 }, fact(started.run))],
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
    reject_with_atomic_work(ledger, |run| {
        HistoryFact::RecoveryProgress(invalid_evidence(run))
    });
}

pub(crate) fn zero_affected_run_windowless_summary_rolls_back_work<
    L: RunHistory + RecordingLedger,
>(
    ledger: &L,
) {
    reject_with_atomic_work(ledger, |run| HistoryFact::RecoverySummary {
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
    reject_with_atomic_work(ledger, |_| HistoryFact::MissingRetentionHistory {
        intent_id: 0,
    });
}
