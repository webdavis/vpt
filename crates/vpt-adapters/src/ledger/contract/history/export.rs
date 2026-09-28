use super::{committed, step};
use vpt_application::ports::{RunHistory, WorkDelta};
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportFailureClass, ExportPosition, HistoryFact, ObservedLogFile,
    PartialAppend, RecoveryEvidence, RunKind, WorkOutcome,
};
use vpt_domain::time::UtcInstant;

pub(crate) fn export_failure_retains_cursor_and_partial_repair<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let cursor = ExportPosition {
        device: 4,
        inode: 5,
        end_offset: 80,
    };
    committed(ledger.acknowledge_export(
        started.records[0].id,
        None,
        cursor,
        UtcInstant { secs: 11 },
    ));
    let event = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[step(12, WorkOutcome::Completed)],
    ))
    .records[0]
        .id;
    let partial = PartialAppend {
        original_end: 97,
        retained_end: 80,
        tail_repair_synced: true,
        bytes_written: 11,
        file_synced: false,
        directory_synced: false,
        identity_revalidated: false,
    };
    let failure = ExportFailure {
        event,
        observed_at: UtcInstant { secs: 12 },
        class: ExportFailureClass::Io,
        last_confirmed: Some(cursor),
        observed_file: None,
        partial: Some(partial),
    };
    committed(ledger.note_export_failure(&failure));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(
        view.failures
            .iter()
            .map(|saved| &saved.failure)
            .collect::<Vec<_>>(),
        vec![&failure]
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn recovery_progress_and_summary_are_saved_facts<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let run = started.run;
    let baseline = vpt_domain::run::RecoveryBaseline {
        generation: None,
        pending_from: Some(started.records[0].id),
        through: Some(started.records[0].id),
    };
    let evidence = RecoveryEvidence {
        affected_runs: vec![run],
        first_failure_observed_at: Some(UtcInstant { secs: 2 }),
        logging_restored_observed_at: None,
        failures: 2,
        recovered_records: 1,
        completed_work: 0,
        failed_work: 0,
        unknown_work: 1,
        deferred_work: 0,
        completed_effects: 0,
        failed_effects: 0,
        unresolved_effects: 0,
        missing_history: 0,
    };
    committed(ledger.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 11 },
            HistoryFact::RecoveryProgress {
                evidence: evidence.clone(),
                baseline,
            },
        )],
    ));
    let progress = ledger
        .recovery_view()
        .unwrap()
        .records
        .last()
        .unwrap()
        .fact
        .clone();
    assert_eq!(
        progress,
        HistoryFact::RecoveryProgress {
            evidence: evidence.clone(),
            baseline,
        }
    );
    let mut restored = evidence;
    restored.logging_restored_observed_at = Some(UtcInstant { secs: 12 });
    restored.recovered_records = 2;
    committed(ledger.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 12 },
            HistoryFact::RecoverySummary {
                evidence: restored.clone(),
                window: None,
            },
        )],
    ));
    assert_eq!(
        ledger.recovery_view().unwrap().records.last().unwrap().fact,
        HistoryFact::RecoverySummary {
            evidence: restored,
            window: None
        }
    );
}

pub(crate) fn failure_immediately_after_reset_has_no_confirmed_position<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let original = ExportPosition {
        device: 4,
        inode: 5,
        end_offset: 80,
    };
    committed(ledger.acknowledge_export(
        started.records[0].id,
        None,
        original,
        UtcInstant { secs: 12 },
    ));
    let old_cursor = ExportCursor {
        generation: 0,
        position: original,
    };
    let observed = ExportPosition {
        device: 4,
        inode: 6,
        end_offset: 0,
    };
    committed(ledger.restart_export(old_cursor, observed, UtcInstant { secs: 13 }));
    let inspection = ObservedLogFile {
        device: 4,
        inode: 6,
        complete_end_offset: 0,
        physical_end_offset: 7,
    };
    let first_failure = ExportFailure {
        event: started.records[0].id,
        observed_at: UtcInstant { secs: 14 },
        class: ExportFailureClass::Io,
        last_confirmed: None,
        observed_file: Some(inspection),
        partial: None,
    };
    committed(ledger.note_export_failure(&first_failure));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.failures.len(), 1);
    assert_eq!(view.failures[0].failure, first_failure);
    assert_eq!(view.records[0].id, started.records[0].id);
    assert_eq!(view.records[0].export_position, None);
    assert_eq!(
        view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: observed
        })
    );
}

pub(crate) fn failure_after_first_replay_ack_uses_receipt_position<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let second = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[step(11, WorkOutcome::Completed)],
    ))
    .records[0]
        .id;
    let original = ExportPosition {
        device: 4,
        inode: 5,
        end_offset: 80,
    };
    committed(ledger.acknowledge_export(
        started.records[0].id,
        None,
        original,
        UtcInstant { secs: 12 },
    ));
    let observed = ExportPosition {
        device: 4,
        inode: 6,
        end_offset: 0,
    };
    committed(ledger.restart_export(
        ExportCursor {
            generation: 0,
            position: original,
        },
        observed,
        UtcInstant { secs: 13 },
    ));
    let replay = ExportPosition {
        end_offset: 90,
        ..observed
    };
    committed(ledger.acknowledge_export(
        started.records[0].id,
        Some(ExportCursor {
            generation: 1,
            position: observed,
        }),
        replay,
        UtcInstant { secs: 15 },
    ));
    let inspection = ObservedLogFile {
        device: 4,
        inode: 6,
        complete_end_offset: 0,
        physical_end_offset: 7,
    };
    let second_failure = ExportFailure {
        event: second,
        last_confirmed: Some(replay),
        observed_at: UtcInstant { secs: 16 },
        class: ExportFailureClass::Io,
        observed_file: Some(inspection),
        partial: None,
    };
    committed(ledger.note_export_failure(&second_failure));
    let view = ledger.recovery_view().unwrap();
    assert_eq!(view.failures.len(), 1);
    assert_eq!(view.failures[0].failure, second_failure);
    assert_eq!(view.records[0].id, started.records[0].id);
    assert_eq!(view.records[0].export_position, Some(replay));
    assert_eq!(view.records[1].id, second);
    assert_eq!(view.records[1].export_position, None);
    assert_eq!(
        view.active_cursor,
        Some(ExportCursor {
            generation: 1,
            position: replay
        })
    );
}

pub(crate) fn encoding_export_failure_stays_distinct_from_io<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let failure = ExportFailure {
        event: started.records[0].id,
        observed_at: UtcInstant { secs: 11 },
        class: ExportFailureClass::Encoding,
        last_confirmed: None,
        observed_file: None,
        partial: None,
    };
    committed(ledger.note_export_failure(&failure));
    assert_eq!(ledger.recovery_view().unwrap().failures[0].failure, failure);
}
