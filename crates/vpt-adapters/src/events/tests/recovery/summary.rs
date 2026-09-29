use super::*;
use vpt_domain::run::{EffectKind, EffectReference, WorkOutcome};

mod replay;

#[test]
fn saved_summary_watermark_prevents_recounting_prior_failures() {
    let fixture = Fixture::new();
    let (receipt, failure) = saved_log_failure(&fixture);
    let before = fixture.history.recovery_view().unwrap();

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    let (evidence, window) = summary_fact(summaries(&after)[0]);
    assert_eq!(
        window,
        Some(FailureWindow {
            after: None,
            through: failure.id
        })
    );
    assert_eq!(after.summarized_through, Some(failure.id));
    assert_eq!(evidence.affected_runs, vec![receipt.run]);
    assert_eq!(
        evidence.first_failure_observed_at,
        Some(failure.failure.observed_at)
    );
    assert_eq!(evidence.logging_restored_observed_at, Some(at(20)));
    assert_eq!(evidence.failures, 1);
    assert_eq!(evidence.recovered_records, before.records.len() as u32);
    assert_eq!(evidence.missing_history, 0);
    assert_eq!(
        after
            .records
            .iter()
            .filter(|record| matches!(&record.fact, HistoryFact::RunStarted(RunKind::Recover)))
            .count(),
        1
    );
    assert!(fixture.history.pending_exports().unwrap().is_empty());
    let documents = fixture.documents();
    let summary_document = documents
        .iter()
        .find(|doc| doc["fact"]["tag"] == "recovery_summary")
        .unwrap();
    assert_eq!(
        summary_document["fact"]["window"]["after"],
        serde_json::Value::Null
    );
    assert_eq!(
        summary_document["fact"]["window"]["through"],
        failure.id.render()
    );

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&after),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fixture.history.recovery_view().unwrap(), after);
    assert_eq!(fixture.documents(), documents);
}

#[test]
fn work_only_recovery_summary_leaves_failure_watermark_unchanged() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Ingest, at(10))).run;
    committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            at(11),
            HistoryFact::WorkStep {
                recording: None,
                outcome: WorkOutcome::Unknown,
            },
        )],
    ));
    let before = fixture.history.recovery_view().unwrap();
    assert!(before.failures.is_empty());

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    let (evidence, window) = summary_fact(summaries(&after)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.affected_runs, vec![run]);
    assert_eq!(evidence.first_failure_observed_at, None);
    assert_eq!(evidence.failures, 0);
    assert_eq!(evidence.unknown_work, 1);
    assert_eq!(evidence.unresolved_effects, 0);
    assert_eq!(after.summarized_through, None);
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&after),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert_eq!(fixture.history.recovery_view().unwrap(), after);
}

#[test]
fn unresolved_effects_are_reported_without_operation_retry() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Ingest, at(10))).run;
    committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[
            (
                at(11),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Completed,
                },
            ),
            (
                at(12),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Failed,
                },
            ),
            (
                at(13),
                HistoryFact::EffectIntent {
                    kind: EffectKind::Archive,
                    reference: EffectReference::ArchiveTarget(fixture.path("archive-target")),
                },
            ),
        ],
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
    let (evidence, window) = summary_fact(summaries(&after)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.completed_work, 1);
    assert_eq!(evidence.failed_work, 1);
    assert_eq!(evidence.unknown_work, 0);
    assert_eq!(evidence.completed_effects, 0);
    assert_eq!(evidence.failed_effects, 0);
    assert_eq!(evidence.unresolved_effects, 1);
    assert_eq!(evidence.missing_history, 0);
    assert_eq!(
        after
            .records
            .iter()
            .filter(|record| matches!(
                &record.fact,
                HistoryFact::WorkStep { .. } | HistoryFact::EffectIntent { .. }
            ))
            .map(|record| (&record.id, &record.fact))
            .collect::<Vec<_>>(),
        before
            .records
            .iter()
            .filter(|record| matches!(
                &record.fact,
                HistoryFact::WorkStep { .. } | HistoryFact::EffectIntent { .. }
            ))
            .map(|record| (&record.id, &record.fact))
            .collect::<Vec<_>>()
    );
    assert!(
        !after
            .records
            .iter()
            .any(|record| matches!(&record.fact, HistoryFact::EffectObserved { .. }))
    );
}
