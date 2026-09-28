use super::*;
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, WorkOutcome};

fn archive_intent(fixture: &Fixture, name: &str) -> HistoryFact {
    HistoryFact::EffectIntent {
        kind: EffectKind::Archive,
        reference: EffectReference::ArchiveTarget(fixture.path(name)),
    }
}

#[test]
fn independent_work_and_effect_facts_keep_separate_recovery_counts() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Ingest, at(10))).run;
    let first = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[
            (
                at(11),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Unknown,
                },
            ),
            (
                at(11),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Completed,
                },
            ),
            (at(11), archive_intent(&fixture, "unresolved-archive")),
            (at(11), archive_intent(&fixture, "completed-archive")),
        ],
    ));
    let unresolved_intent = first.records[2].id;
    let completed_intent = first.records[3].id;
    assert_ne!(unresolved_intent, completed_intent);
    committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[(
            at(12),
            HistoryFact::EffectObserved {
                intent: completed_intent,
                outcome: EffectOutcome::Completed,
            },
        )],
    ));

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&view).len(), 1);
    let (evidence, window) = summary_fact(summaries(&view)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.affected_runs, vec![run]);
    assert_eq!(evidence.completed_work, 1);
    assert_eq!(evidence.failed_work, 0);
    assert_eq!(evidence.unknown_work, 1);
    assert_eq!(evidence.completed_effects, 1);
    assert_eq!(evidence.failed_effects, 0);
    assert_eq!(evidence.unresolved_effects, 1);
}

#[test]
fn latest_saved_effect_observation_controls_recovery_classification() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Ingest, at(10))).run;
    let intents = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[
            (at(11), archive_intent(&fixture, "failed-archive")),
            (at(11), archive_intent(&fixture, "unknown-archive")),
        ],
    ));
    let failed_intent = intents.records[0].id;
    let unknown_intent = intents.records[1].id;
    let observations = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[
            (
                at(12),
                HistoryFact::EffectObserved {
                    intent: failed_intent,
                    outcome: EffectOutcome::Unknown,
                },
            ),
            (
                at(12),
                HistoryFact::EffectObserved {
                    intent: failed_intent,
                    outcome: EffectOutcome::Completed,
                },
            ),
            (
                at(12),
                HistoryFact::EffectObserved {
                    intent: failed_intent,
                    outcome: EffectOutcome::Failed,
                },
            ),
            (
                at(12),
                HistoryFact::EffectObserved {
                    intent: unknown_intent,
                    outcome: EffectOutcome::Completed,
                },
            ),
            (
                at(12),
                HistoryFact::EffectObserved {
                    intent: unknown_intent,
                    outcome: EffectOutcome::Unknown,
                },
            ),
        ],
    ));
    assert!(
        observations
            .records
            .windows(2)
            .all(|pair| pair[0].id.sequence() < pair[1].id.sequence())
    );

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&view).len(), 1);
    let (evidence, window) = summary_fact(summaries(&view)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.completed_effects, 0);
    assert_eq!(evidence.failed_effects, 1);
    assert_eq!(evidence.unresolved_effects, 1);
    assert_eq!(evidence.completed_work, 0);
    assert_eq!(evidence.failed_work, 0);
    assert_eq!(evidence.unknown_work, 0);
}

#[test]
fn deferred_work_is_distinct_from_unknown_and_completed_recovery_work() {
    let fixture = Fixture::new();
    let run = committed(fixture.history.begin_run(RunKind::Ingest, at(10))).run;
    let saved = committed(fixture.history.commit_step(
        run,
        &WorkDelta::default(),
        &[
            (
                at(11),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Deferred,
                },
            ),
            (
                at(12),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Unknown,
                },
            ),
            (
                at(13),
                HistoryFact::WorkStep {
                    recording: None,
                    outcome: WorkOutcome::Completed,
                },
            ),
        ],
    ));
    assert_eq!(
        sink(&fixture, &[], &[]).recover(&fixture.history.recovery_view().unwrap()),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(summaries(&view).len(), 1);
    let (evidence, window) = summary_fact(summaries(&view)[0]);
    assert_eq!(window, None);
    assert_eq!(evidence.affected_runs, vec![run]);
    assert_eq!(evidence.completed_work, 1);
    assert_eq!(evidence.failed_work, 0);
    assert_eq!(evidence.unknown_work, 1);
    assert_eq!(evidence.completed_effects, 0);
    assert_eq!(evidence.failed_effects, 0);
    assert_eq!(evidence.unresolved_effects, 0);
    assert_eq!(
        view.records
            .iter()
            .filter(
                |record| record.run == run && matches!(record.fact, HistoryFact::WorkStep { .. })
            )
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        saved
            .records
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>()
    );
    let summary = fixture
        .documents()
        .into_iter()
        .find(|document| document["fact"]["tag"] == "recovery_summary")
        .unwrap();
    assert_eq!(summary["fact"]["evidence"]["deferred_work"], 1);
    assert_eq!(summary["fact"]["evidence"]["unknown_work"], 1);
}
