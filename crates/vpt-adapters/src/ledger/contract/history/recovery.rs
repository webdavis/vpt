use super::{committed, record};
use std::path::PathBuf;
use vpt_application::ports::{HistoryWrite, LedgerCommit, RecordingLedger, RunHistory, WorkDelta};
use vpt_domain::history::{EventId, ExportFailureId, LedgerNamespace};
use vpt_domain::run::{
    ExportFailure, ExportFailureClass, FailureWindow, HistoryFact, ObservedLogFile,
    RecoveryBaseline, RecoveryEvidence, RunKind,
};
use vpt_domain::time::UtcInstant;

mod baseline;
pub(crate) use baseline::{
    atomic_recovery_begin_on_empty_ledger_has_no_frontier,
    atomic_recovery_begin_retains_all_acked_frontier,
    atomic_recovery_begin_snapshots_pending_frontier,
    first_recovery_progress_rejects_fabricated_snapshot,
    foreign_pending_recovery_baseline_rolls_back_work,
    later_recovery_progress_keeps_first_snapshot_after_ack,
    missing_pending_recovery_baseline_rolls_back_work, reversed_recovery_baseline_rolls_back_work,
    zero_pending_recovery_baseline_rolls_back_work,
};

fn failure(event: EventId) -> ExportFailure {
    ExportFailure {
        event,
        observed_at: UtcInstant { secs: 12 },
        class: ExportFailureClass::Io,
        last_confirmed: None,
        observed_file: None,
        partial: None,
    }
}

fn evidence(run: vpt_domain::history::RunId) -> RecoveryEvidence {
    RecoveryEvidence {
        affected_runs: vec![run],
        first_failure_observed_at: Some(UtcInstant { secs: 12 }),
        logging_restored_observed_at: None,
        failures: 1,
        recovered_records: 0,
        completed_work: 0,
        failed_work: 0,
        unknown_work: 1,
        deferred_work: 0,
        completed_effects: 0,
        failed_effects: 0,
        unresolved_effects: 0,
        missing_history: 0,
    }
}

fn summary(evidence: RecoveryEvidence, window: Option<FailureWindow>) -> HistoryFact {
    HistoryFact::RecoverySummary { evidence, window }
}

pub(crate) fn work_only_recovery_summary_preserves_absent_failure_time<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let evidence = RecoveryEvidence {
        first_failure_observed_at: None,
        failures: 0,
        ..evidence(started.run)
    };
    let fact = summary(evidence, None);
    committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 11 }, fact.clone())],
    ));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records.last().unwrap().fact, fact);
    assert_eq!(view.summarized_through, None);
    assert!(view.failures.is_empty());
}

pub(crate) fn saved_export_failures_have_unique_order_for_repeated_observation<L: RunHistory>(
    ledger: &L,
) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let failure = failure(started.records[0].id);
    committed(ledger.note_export_failure(&failure));
    committed(ledger.note_export_failure(&failure));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.failures.len(), 2);
    assert_eq!(view.failures[0].failure, failure);
    assert_eq!(view.failures[1].failure, failure);
    assert_eq!(view.failures[0].id.namespace(), started.run.namespace());
    assert_eq!(view.failures[1].id.namespace(), started.run.namespace());
    assert_ne!(view.failures[0].id, view.failures[1].id);
    assert!(view.failures[0].id.sequence() < view.failures[1].id.sequence());
}

pub(crate) fn observed_log_file_round_trips_without_acknowledgement<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let observed = ObservedLogFile {
        device: 4,
        inode: 5,
        complete_end_offset: 80,
        physical_end_offset: 97,
    };
    let failure = ExportFailure {
        observed_file: Some(observed),
        ..failure(started.records[0].id)
    };
    committed(ledger.note_export_failure(&failure));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.failures[0].failure.observed_file, Some(observed));
    assert_eq!(view.failures[0].failure, failure);
    assert!(view.active_cursor.is_none());
    assert_eq!(
        ledger.pending_exports().unwrap()[0].id,
        started.records[0].id
    );
}

pub(crate) fn recovery_progress_does_not_advance_failure_watermark<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let failure = failure(started.records[0].id);
    committed(ledger.note_export_failure(&failure));
    committed(ledger.note_export_failure(&failure));
    let progress = HistoryFact::RecoveryProgress {
        evidence: evidence(started.run),
        baseline: RecoveryBaseline {
            generation: None,
            pending_from: Some(started.records[0].id),
            through: Some(started.records[0].id),
        },
    };
    committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 13 }, progress)],
    ));
    assert_eq!(ledger.recovery_view().unwrap().summarized_through, None);
}

pub(crate) fn recovery_summaries_advance_saved_failure_window<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let failure = failure(started.records[0].id);
    committed(ledger.note_export_failure(&failure));
    committed(ledger.note_export_failure(&failure));
    let namespace = started.run.namespace();
    let first_id = ExportFailureId::new(namespace, 1);
    let second_id = ExportFailureId::new(namespace, 2);
    let first = FailureWindow {
        after: None,
        through: first_id,
    };
    committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 14 },
            summary(evidence(started.run), Some(first)),
        )],
    ));
    let first_view = ledger.recovery_view().unwrap();
    assert_eq!(first_view.summarized_through, Some(first.through));
    assert_eq!(first_view.failures[0].id, first_id);
    assert_eq!(first_view.failures[0].failure, failure);
    assert_eq!(first_view.failures[1].id, second_id);
    assert_eq!(first_view.failures[1].failure, failure);
    let second = FailureWindow {
        after: Some(first.through),
        through: second_id,
    };
    committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 15 },
            summary(evidence(started.run), Some(second)),
        )],
    ));
    let second_view = ledger.recovery_view().unwrap();
    assert_eq!(second_view.summarized_through, Some(second.through));
    assert_eq!(second_view.failures[0].id, first_id);
    assert_eq!(second_view.failures[0].failure, failure);
    assert_eq!(second_view.failures[1].id, second_id);
    assert_eq!(second_view.failures[1].failure, failure);
}

pub(crate) fn summary_window_rejects_stale_missing_and_foreign_ids<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let failure = failure(started.records[0].id);
    committed(ledger.note_export_failure(&failure));
    committed(ledger.note_export_failure(&failure));
    let failures = ledger.recovery_view().unwrap().failures;
    let namespace = started.run.namespace();
    for window in [
        FailureWindow {
            after: Some(failures[0].id),
            through: failures[1].id,
        },
        FailureWindow {
            after: None,
            through: ExportFailureId::new(namespace, failures[1].id.sequence() + 1),
        },
        FailureWindow {
            after: None,
            through: ExportFailureId::new(vpt_domain::history::LedgerNamespace::new([0x77; 16]), 1),
        },
    ] {
        let result = ledger.commit_step(
            started.run,
            &WorkDelta::default(),
            &[(
                UtcInstant { secs: 13 },
                summary(evidence(started.run), Some(window)),
            )],
        );
        assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    }
    assert_eq!(ledger.recovery_view().unwrap().summarized_through, None);
    let first = FailureWindow {
        after: None,
        through: failures[0].id,
    };
    committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 14 },
            summary(evidence(started.run), Some(first)),
        )],
    ));
    let stale = FailureWindow {
        after: None,
        through: failures[1].id,
    };
    let result = ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 15 },
            summary(evidence(started.run), Some(stale)),
        )],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(
        ledger.recovery_view().unwrap().summarized_through,
        Some(first.through)
    );
}

pub(crate) fn invalid_summary_window_rolls_back_work_delta<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    let recording = record(94);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let original = ledger.by_id(&recording.id).unwrap().unwrap().source_path;
    let invalid = FailureWindow {
        after: None,
        through: ExportFailureId::new(started.run.namespace(), 999),
    };
    let result = ledger.commit_step(
        started.run,
        &WorkDelta {
            source_updates: vec![(
                recording.id.clone(),
                PathBuf::from("/vm/Recordings/changed-94.m4a"),
            )],
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 11 },
            summary(evidence(started.run), Some(invalid)),
        )],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(
        ledger.by_id(&recording.id).unwrap().unwrap().source_path,
        original
    );
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.records.len(), 1);
    assert_eq!(view.summarized_through, None);
}
