use super::*;
use vpt_domain::run::{ExportPosition, WorkOutcome};

pub(crate) fn atomic_recovery_begin_snapshots_pending_frontier<L: RunHistory>(ledger: &L) {
    let work = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    let step = committed(ledger.commit_step(
        work.run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 11 },
            HistoryFact::WorkStep {
                recording: None,
                outcome: WorkOutcome::Unknown,
            },
        )],
    ));
    committed(ledger.acknowledge_export(
        work.records[0].id,
        None,
        ExportPosition {
            device: 4,
            inode: 5,
            end_offset: 20,
        },
        UtcInstant { secs: 12 },
    ));
    let expected = evidence(work.run);
    let started = committed(ledger.begin_recovery(UtcInstant { secs: 13 }, expected.clone()));
    assert_eq!(started.records.len(), 2);
    assert_eq!(
        started.records[0].fact,
        HistoryFact::RunStarted(RunKind::Recover)
    );
    assert_eq!(
        started.records[1].fact,
        HistoryFact::RecoveryProgress {
            evidence: expected,
            baseline: RecoveryBaseline {
                generation: Some(0),
                pending_from: Some(step.records[0].id),
                through: Some(step.records[0].id),
            },
        }
    );
    assert_eq!(started.records[0].run, started.run);
    assert_eq!(started.records[1].run, started.run);
    assert!(started.records[0].id.sequence() < started.records[1].id.sequence());
    let view = ledger.recovery_view().unwrap();
    assert_eq!(&view.records[2..], started.records.as_slice());
    assert_eq!(view.summarized_through, None);
    assert_eq!(
        ledger
            .pending_exports()
            .unwrap()
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        [
            step.records[0].id,
            started.records[0].id,
            started.records[1].id
        ]
    );
}

pub(crate) fn atomic_recovery_begin_retains_all_acked_frontier<L: RunHistory>(ledger: &L) {
    let work = committed(ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }));
    committed(ledger.acknowledge_export(
        work.records[0].id,
        None,
        ExportPosition {
            device: 4,
            inode: 5,
            end_offset: 20,
        },
        UtcInstant { secs: 11 },
    ));
    let started = committed(ledger.begin_recovery(UtcInstant { secs: 12 }, evidence(work.run)));
    assert_eq!(started.records.len(), 2);
    assert_eq!(
        started.records[1].fact,
        HistoryFact::RecoveryProgress {
            evidence: evidence(work.run),
            baseline: RecoveryBaseline {
                generation: Some(0),
                pending_from: None,
                through: Some(work.records[0].id),
            },
        }
    );
    let view = ledger.recovery_view().unwrap();
    assert_eq!(&view.records[1..], started.records.as_slice());
    assert_eq!(ledger.pending_exports().unwrap(), started.records);
}

pub(crate) fn atomic_recovery_begin_on_empty_ledger_has_no_frontier<L: RunHistory>(ledger: &L) {
    let evidence = RecoveryEvidence {
        affected_runs: vec![],
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
    };
    let started = committed(ledger.begin_recovery(UtcInstant { secs: 10 }, evidence.clone()));
    assert_eq!(started.records.len(), 2);
    assert_eq!(
        started.records[1].fact,
        HistoryFact::RecoveryProgress {
            evidence,
            baseline: RecoveryBaseline::default(),
        }
    );
    assert_eq!(ledger.recovery_view().unwrap().records, started.records);
}

pub(crate) fn invalid_recovery_baseline_rolls_back_work_delta<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    let recording = record(95);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let before = ledger.recovery_view().unwrap();
    let original = ledger.by_id(&recording.id).unwrap().unwrap().source_path;
    let namespace = started.run.namespace();
    for baseline in [
        RecoveryBaseline {
            generation: Some(0),
            pending_from: Some(EventId::new(namespace, 3)),
            through: Some(EventId::new(namespace, 2)),
        },
        RecoveryBaseline {
            generation: None,
            pending_from: None,
            through: Some(started.records[0].id),
        },
        RecoveryBaseline {
            generation: None,
            pending_from: Some(EventId::new(namespace, 0)),
            through: Some(started.records[0].id),
        },
        RecoveryBaseline {
            generation: None,
            pending_from: Some(EventId::new(LedgerNamespace::new([0x91; 16]), 1)),
            through: Some(started.records[0].id),
        },
    ] {
        let result = ledger.commit_step(
            started.run,
            &WorkDelta {
                source_updates: vec![(
                    recording.id.clone(),
                    PathBuf::from("/vm/Recordings/changed-95.m4a"),
                )],
                ..Default::default()
            },
            &[(
                UtcInstant { secs: 11 },
                HistoryFact::RecoveryProgress {
                    evidence: evidence(started.run),
                    baseline,
                },
            )],
        );
        assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
        assert_eq!(
            ledger.by_id(&recording.id).unwrap().unwrap().source_path,
            original
        );
        assert_eq!(ledger.recovery_view().unwrap(), before);
    }
}

pub(crate) fn first_recovery_progress_rejects_fabricated_snapshot<
    L: RunHistory + RecordingLedger,
>(
    ledger: &L,
) {
    let recording = record(96);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let before = ledger.recovery_view().unwrap();
    assert_eq!(before.records, started.records);
    let result = ledger.commit_step(
        started.run,
        &WorkDelta {
            source_updates: vec![(
                recording.id.clone(),
                PathBuf::from("/vm/Recordings/fabricated.m4a"),
            )],
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 11 },
            HistoryFact::RecoveryProgress {
                evidence: evidence(started.run),
                baseline: RecoveryBaseline::default(),
            },
        )],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(ledger.recovery_view().unwrap(), before);
    assert_eq!(
        ledger.by_id(&recording.id).unwrap().unwrap().source_path,
        recording.source_path
    );
}

pub(crate) fn later_recovery_progress_keeps_first_snapshot_after_ack<L: RunHistory>(ledger: &L) {
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let baseline = RecoveryBaseline {
        generation: None,
        pending_from: Some(started.records[0].id),
        through: Some(started.records[0].id),
    };
    let fact = HistoryFact::RecoveryProgress {
        evidence: evidence(started.run),
        baseline,
    };
    let first = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 11 }, fact.clone())],
    ));
    let first_position = ExportPosition {
        device: 4,
        inode: 5,
        end_offset: 20,
    };
    let second_position = ExportPosition {
        device: 4,
        inode: 5,
        end_offset: 40,
    };
    committed(ledger.acknowledge_export(
        started.records[0].id,
        None,
        first_position,
        UtcInstant { secs: 12 },
    ));
    committed(ledger.acknowledge_export(
        first.records[0].id,
        Some(vpt_domain::run::ExportCursor {
            generation: 0,
            position: first_position,
        }),
        second_position,
        UtcInstant { secs: 12 },
    ));
    let continued = committed(ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(UtcInstant { secs: 13 }, fact)],
    ));
    let before = ledger.recovery_view().unwrap();
    assert_eq!(
        continued.records[0].fact,
        HistoryFact::RecoveryProgress {
            evidence: evidence(started.run),
            baseline,
        }
    );
    let changed = RecoveryBaseline {
        generation: Some(0),
        pending_from: Some(continued.records[0].id),
        through: Some(continued.records[0].id),
    };
    let refused = ledger.commit_step(
        started.run,
        &WorkDelta::default(),
        &[(
            UtcInstant { secs: 14 },
            HistoryFact::RecoveryProgress {
                evidence: evidence(started.run),
                baseline: changed,
            },
        )],
    );
    assert!(
        matches!(refused, HistoryWrite::Unconfirmed(_)),
        "{refused:?}"
    );
    assert_eq!(ledger.recovery_view().unwrap(), before);
}
