use super::super::failures::RefusingHistory;
use super::*;
use vpt_application::ports::{LedgerError, SinkError};
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, WorkOutcome};
use vpt_protocol::encode_history;

#[test]
fn interrupted_summary_commit_append_and_ack_resume_one_saved_summary() {
    let fixture = Fixture::new();
    let (receipt, failure) = saved_log_failure(&fixture);
    let recovery = committed(fixture.history.begin_run(RunKind::Recover, at(30)));
    let before_progress = fixture.history.recovery_view().unwrap();
    let progress = HistoryFact::RecoveryProgress {
        evidence: RecoveryEvidence {
            logging_restored_observed_at: None,
            recovered_records: 0,
            ..evidence(receipt.run, &failure)
        },
        baseline: vpt_domain::run::RecoveryBaseline {
            generation: before_progress
                .active_cursor
                .map(|cursor| cursor.generation),
            pending_from: Some(before_progress.records[0].id),
            through: Some(recovery.records[0].id),
        },
    };
    let progress_receipt = committed(fixture.history.commit_step(
        recovery.run,
        &WorkDelta::default(),
        &[(at(31), progress.clone())],
    ));
    let before = fixture.history.recovery_view().unwrap();

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(summaries(&after)[0].run, recovery.run);
    assert_eq!(after.summarized_through, Some(failure.id));
    assert_eq!(
        after
            .records
            .iter()
            .filter(|record| matches!(&record.fact, HistoryFact::RunStarted(RunKind::Recover)))
            .count(),
        1
    );
    assert_eq!(
        &after
            .records
            .iter()
            .find(|record| record.id == progress_receipt.records[0].id)
            .unwrap()
            .fact,
        &progress
    );
    assert_eq!(fixture.history.pending_exports().unwrap(), []);
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&after),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fixture.history.recovery_view().unwrap(), after);
}

#[test]
fn already_saved_summary_reexports_without_creating_another() {
    let fixture = Fixture::new();
    let (receipt, failure) = saved_log_failure(&fixture);
    assert_eq!(
        sink(&fixture, &[], &[]).after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let recovery = committed(fixture.history.begin_run(RunKind::Recover, at(30)));
    assert_eq!(
        sink(&fixture, &[], &[]).after_commit(&recovery),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let saved = committed(fixture.history.commit_step(
        recovery.run,
        &WorkDelta::default(),
        &[
            (
                at(31),
                HistoryFact::RecoverySummary {
                    evidence: evidence(receipt.run, &failure),
                    window: Some(FailureWindow {
                        after: None,
                        through: failure.id,
                    }),
                },
            ),
            (at(32), HistoryFact::RunFinished(RunOutcome::Completed)),
        ],
    ));
    let summary = &saved.records[0];
    let line = encode_history(summary, at(40)).unwrap();
    fixture
        .state
        .open_event_log()
        .unwrap()
        .append_line(&line)
        .unwrap();
    let before = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&before).len(), 1);
    assert_eq!(before.summarized_through, Some(failure.id));
    assert_eq!(before.records.last().unwrap().export_position, None);

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(after.records.len(), before.records.len());
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(summaries(&after)[0].id, summary.id);
    assert_eq!(after.summarized_through, Some(failure.id));
    assert!(fixture.history.pending_exports().unwrap().is_empty());
    let documents = fixture.documents();
    assert_eq!(
        documents
            .iter()
            .filter(|doc| doc["event_id"] == summary.id.render())
            .count(),
        2
    );
    assert_eq!(
        documents
            .iter()
            .filter(|doc| doc["fact"]["tag"] == "recovery_summary")
            .count(),
        2
    );
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&after),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fixture.documents(), documents);
    assert_eq!(fixture.history.recovery_view().unwrap(), after);
}

#[test]
fn new_reconciliation_fact_after_summary_creates_updated_windowless_summary() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Ingest, at(10))).run;
    let intent = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            at(11),
            HistoryFact::EffectIntent {
                kind: EffectKind::Archive,
                reference: EffectReference::ArchiveTarget(fixture.path("archive-target")),
            },
        )],
    ));
    let recovery = committed(fixture.history.begin_run(RunKind::Recover, at(20)));
    let saved = committed(fixture.history.commit_step(
        recovery.run,
        &WorkDelta::default(),
        &[(
            at(21),
            HistoryFact::RecoverySummary {
                evidence: RecoveryEvidence {
                    affected_runs: vec![run],
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
                    unresolved_effects: 1,
                    missing_history: 0,
                },
                window: None,
            },
        )],
    ));
    let prior_summary_id = saved.records[0].id;
    let before = fixture.history.recovery_view().unwrap();
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let unchanged = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&unchanged).len(), 1);
    assert_eq!(summaries(&unchanged)[0].id, prior_summary_id);

    let recording =
        vpt_domain::identity::RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap();
    let observed = committed(fixture.history.commit_step(
        recovery.run,
        &WorkDelta::default(),
        &[
            (
                at(11),
                HistoryFact::EffectObserved {
                    intent: intent.records[0].id,
                    outcome: EffectOutcome::Completed,
                },
            ),
            (
                at(11),
                HistoryFact::WorkStep {
                    recording: Some(recording),
                    outcome: WorkOutcome::Completed,
                },
            ),
        ],
    ));
    assert_eq!(observed.run, recovery.run);
    assert!(observed.records[0].id.sequence() > prior_summary_id.sequence());
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 2);
    let (evidence, window) = summary_fact(summaries(&after)[1]);
    assert_eq!(window, None);
    assert_eq!(evidence.affected_runs, vec![run, recovery.run]);
    assert_eq!(evidence.completed_work, 1);
    assert_eq!(evidence.completed_effects, 1);
    assert_eq!(evidence.unresolved_effects, 0);
    assert_eq!(after.summarized_through, None);
    assert_eq!(
        after
            .records
            .iter()
            .filter(|record| matches!(&record.fact, HistoryFact::EffectObserved { .. }))
            .count(),
        1
    );
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&after),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fixture.history.recovery_view().unwrap(), after);
}

#[test]
fn required_history_refusal_cannot_invent_missing_facts() {
    let fixture = Fixture::new();
    fs::write(fixture.log_path(), b"retained\n").unwrap();
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o644)).unwrap();
    let original = fs::read(fixture.log_path()).unwrap();
    let refusing = RefusingHistory(LedgerError::Busy);
    let mut sink = LoggedEvents::new(&fixture.state, &refusing, &[], &[], || at(20));

    assert!(matches!(
        sink.recover(&vpt_domain::run::RecoveryView::default()),
        SinkOutcome::Stop(stop)
            if stop.reason == SinkError::RequiredHistory(LedgerError::Busy)
                && stop.committed.is_none()
                && stop.warnings.is_empty()
    ));
    assert_eq!(fs::read(fixture.log_path()).unwrap(), original);
    assert_eq!(
        fs::metadata(fixture.log_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o600)).unwrap();
    let empty = MemoryLedger::new();
    let before = empty.recovery_view().unwrap();
    assert_eq!(
        LoggedEvents::new(&fixture.state, &empty, &[], &[], || at(20)).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(empty.recovery_view().unwrap(), before);
    assert_eq!(fs::read(fixture.log_path()).unwrap(), original);
}
