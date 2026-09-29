use super::*;

fn seed_interrupted_prefix(
    fixture: &Fixture,
    acknowledged: usize,
    updated: bool,
) -> (
    vpt_domain::history::RunId,
    Vec<vpt_domain::history::EventId>,
) {
    let (receipt, failure) = saved_log_failure(fixture);
    let original = fixture.history.recovery_view().unwrap().records;
    assert_eq!(original.len(), 3);
    let recovery = committed(fixture.history.begin_recovery(
        at(30),
        RecoveryEvidence {
            logging_restored_observed_at: None,
            recovered_records: 0,
            ..evidence(receipt.run, &failure)
        },
    ));
    let HistoryFact::RecoveryProgress { baseline, .. } = &recovery.records[1].fact else {
        panic!("atomic recovery progress");
    };
    assert_eq!(
        *baseline,
        RecoveryBaseline {
            generation: None,
            pending_from: Some(original[0].id),
            through: Some(original[2].id),
        }
    );
    let mut log = fixture.state.open_event_log().unwrap();
    let mut cursor = None;
    for record in original.iter().take(acknowledged) {
        let line = encode_history(record, at(40)).unwrap();
        let appended = log.append_line(&line).unwrap();
        let position = ExportPosition {
            device: appended.device,
            inode: appended.inode,
            end_offset: appended.end_offset,
        };
        committed(
            fixture
                .history
                .acknowledge_export(record.id, cursor, position, at(40)),
        );
        cursor = Some(ExportCursor {
            generation: 0,
            position,
        });
    }
    if updated {
        committed(fixture.history.commit_step(
            recovery.run,
            &WorkDelta::default(),
            &[(
                at(41),
                HistoryFact::RecoveryProgress {
                    evidence: RecoveryEvidence {
                        logging_restored_observed_at: Some(at(40)),
                        recovered_records: u32::try_from(acknowledged).unwrap(),
                        ..evidence(receipt.run, &failure)
                    },
                    baseline: *baseline,
                },
            )],
        ));
    }
    (
        recovery.run,
        original.iter().map(|record| record.id).collect(),
    )
}

#[test]
fn interrupted_prefix_ack_before_progress_update_is_counted_on_resume() {
    let fixture = Fixture::new();
    let (run, original) = seed_interrupted_prefix(&fixture, 1, false);
    let before = fixture.history.recovery_view().unwrap();
    assert_eq!(before.records[0].id, original[0]);
    assert!(before.records[0].export_position.is_some());
    assert!(before.records[1].export_position.is_none());
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(summaries(&after)[0].run, run);
    assert_eq!(summary_fact(summaries(&after)[0]).0.recovered_records, 3);
    assert_eq!(
        after
            .records
            .iter()
            .take(3)
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        original
    );
}

#[test]
fn resumed_progress_preserves_prior_confirmed_recovery_count() {
    let fixture = Fixture::new();
    let (run, original) = seed_interrupted_prefix(&fixture, 2, true);
    let before = fixture.history.recovery_view().unwrap();
    assert!(before.records[0].export_position.is_some());
    assert!(before.records[1].export_position.is_some());
    assert!(before.records[2].export_position.is_none());
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(summaries(&after)[0].run, run);
    assert_eq!(summary_fact(summaries(&after)[0]).0.recovered_records, 3);
    assert_eq!(
        after
            .records
            .iter()
            .take(3)
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        original
    );
    let baselines: Vec<_> = after
        .records
        .iter()
        .filter_map(|record| match &record.fact {
            HistoryFact::RecoveryProgress { baseline, .. } if record.run == run => Some(*baseline),
            _ => None,
        })
        .collect();
    assert!(!baselines.is_empty());
    assert!(baselines.iter().all(|baseline| *baseline == baselines[0]));
}

#[test]
fn later_commits_do_not_inflate_fixed_recovery_backlog() {
    let fixture = Fixture::new();
    let (_, original) = seed_interrupted_prefix(&fixture, 1, false);
    let later = committed(fixture.history.begin_run(RunKind::Ingest, at(50)));
    let step = committed(fixture.history.commit_step(
        later.run,
        &WorkDelta::default(),
        &[(
            at(51),
            HistoryFact::WorkStep {
                recording: None,
                outcome: vpt_domain::run::WorkOutcome::Unknown,
            },
        )],
    ));
    assert!(later.records[0].id.sequence() > original[2].sequence());
    assert!(step.records[0].id.sequence() > original[2].sequence());
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    let (evidence, _) = summary_fact(summaries(&after)[0]);
    assert_eq!(evidence.recovered_records, 3);
    assert_eq!(evidence.unknown_work, 1);
    assert!(evidence.affected_runs.contains(&later.run));
    assert!(
        after
            .records
            .iter()
            .take(3)
            .all(|record| record.export_position.is_some())
    );
    assert!(
        after
            .records
            .iter()
            .find(|record| record.id == step.records[0].id)
            .unwrap()
            .export_position
            .is_some()
    );
}
