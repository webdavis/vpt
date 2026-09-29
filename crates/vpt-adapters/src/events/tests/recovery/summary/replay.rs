use super::*;
use std::fs::OpenOptions;
use vpt_domain::run::ExportPosition;

#[test]
fn all_acked_truncation_recovery_counts_confirmed_replay() {
    let fixture = Fixture::new();
    let (receipt, failure) = saved_log_failure(&fixture);
    assert_eq!(
        sink(&fixture, &[], &[]).after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let before = fixture.history.recovery_view().unwrap();
    let original_ids: Vec<_> = before.records.iter().map(|record| record.id).collect();
    assert!(
        before
            .records
            .iter()
            .all(|record| record.export_position.is_some())
    );
    assert!(fixture.history.pending_exports().unwrap().is_empty());
    let old = before.active_cursor.unwrap();
    OpenOptions::new()
        .write(true)
        .open(fixture.log_path())
        .unwrap()
        .set_len(0)
        .unwrap();

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    let (evidence, window) = summary_fact(summaries(&after)[0]);
    assert_eq!(
        window,
        Some(FailureWindow {
            after: None,
            through: failure.id
        })
    );
    assert_eq!(
        evidence.recovered_records,
        u32::try_from(original_ids.len()).unwrap()
    );
    assert_eq!(after.restarts.len(), 1);
    assert_eq!(after.restarts[0].old, old);
    assert_eq!(after.active_cursor.unwrap().generation, old.generation + 1);
    assert_eq!(
        after
            .records
            .iter()
            .take(original_ids.len())
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        original_ids
    );
    assert!(
        after
            .records
            .iter()
            .take(original_ids.len())
            .all(|record| record.export_position.is_some())
    );
}

#[test]
fn all_acked_intact_log_has_no_restored_backlog() {
    let fixture = Fixture::new();
    let (receipt, _) = saved_log_failure(&fixture);
    assert_eq!(
        sink(&fixture, &[], &[]).after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let before = fixture.history.recovery_view().unwrap();
    let cursor = before.active_cursor;
    assert!(
        before
            .records
            .iter()
            .all(|record| record.export_position.is_some())
    );
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(summary_fact(summaries(&after)[0]).0.recovered_records, 0);
    assert_eq!(after.restarts, before.restarts);
    assert_eq!(
        after.active_cursor.unwrap().generation,
        cursor.unwrap().generation
    );
    assert_eq!(
        after
            .records
            .iter()
            .take(before.records.len())
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        before
            .records
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn new_replay_outage_of_same_event_and_timestamp_remains_unsummarized() {
    let fixture = Fixture::new();
    let (receipt, first_failure) = saved_log_failure(&fixture);
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
    let first_summary = committed(fixture.history.commit_step(
        recovery.run,
        &WorkDelta::default(),
        &[
            (
                at(31),
                HistoryFact::RecoverySummary {
                    evidence: evidence(receipt.run, &first_failure),
                    window: Some(FailureWindow {
                        after: None,
                        through: first_failure.id,
                    }),
                },
            ),
            (at(32), HistoryFact::RunFinished(RunOutcome::Completed)),
        ],
    ));
    assert_eq!(
        sink(&fixture, &[], &[]).after_commit(&first_summary),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let before = fixture.history.recovery_view().unwrap();
    assert_eq!(before.summarized_through, Some(first_failure.id));
    let old = before.active_cursor.unwrap();
    OpenOptions::new()
        .write(true)
        .open(fixture.log_path())
        .unwrap()
        .set_len(0)
        .unwrap();
    let inspection = fixture.state.open_event_log().unwrap().inspect().unwrap();
    let observed = ExportPosition {
        device: inspection.device,
        inode: inspection.inode,
        end_offset: inspection.complete_end_offset,
    };
    committed(fixture.history.restart_export(old, observed, at(33)));
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::ExportUnavailable])
        }
    );
    let outage = fixture.history.recovery_view().unwrap();
    assert_eq!(outage.failures.len(), 2);
    let second_failure = &outage.failures[1];
    assert_ne!(first_failure.id, second_failure.id);
    assert_eq!(first_failure.failure.event, second_failure.failure.event);
    assert_eq!(
        first_failure.failure.observed_at,
        second_failure.failure.observed_at
    );
    assert_eq!(outage.summarized_through, Some(first_failure.id));
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o600)).unwrap();

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&outage),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 2);
    let (second_evidence, second_window) = summary_fact(summaries(&after)[1]);
    assert_eq!(
        second_window,
        Some(FailureWindow {
            after: Some(first_failure.id),
            through: second_failure.id,
        })
    );
    assert_eq!(second_evidence.failures, 1);
    assert_eq!(
        second_evidence.first_failure_observed_at,
        Some(second_failure.failure.observed_at)
    );
    assert_eq!(after.summarized_through, Some(second_failure.id));
}
