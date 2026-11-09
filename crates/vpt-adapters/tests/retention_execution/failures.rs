use super::support::*;
use vpt_application::ports::*;
use vpt_application::{ReconcileProgress, Retention, RetentionError, reconcile_intents};
use vpt_domain::run::{EffectOutcome, HistoryFact, WorkOutcome};

#[test]
fn refused_preparation_export_stops_before_trash_and_retains_the_intent() {
    let mut world = World::new();
    let record = world.record("stop.m4a", b"owned stop audio");
    world.sink.stop_after_intent = true;
    let failed = Retention {
        ledger: &world.ledger,
        stores: &world.stores,
        trash: &world.trash,
        clock: &FixedClock,
        sink: &mut world.sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap_err();
    assert_eq!(
        failed.error,
        RetentionError::SinkStopped(SinkError::UnsafeState)
    );
    assert!(failed.committed.is_some());
    assert!(failed.report.moved.is_empty());
    assert!(record.audio_path.exists());
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
    assert!(
        !world
            .sink
            .trace
            .borrow()
            .iter()
            .any(|action| matches!(action, Action::Trash(_)))
    );
    assert_eq!(
        world.sink.trace.borrow().len(),
        2,
        "no export call after the sink stops"
    );
    assert_eq!(
        world.ledger.recovery_view().unwrap().records.len(),
        2,
        "no history fact after the sink stops"
    );
}

#[test]
fn failed_trash_keeps_intent_and_saves_a_failed_observation() {
    failed_helper(
        TrashMode::Failed,
        EffectOutcome::Failed,
        WorkOutcome::Failed,
    );
}

#[test]
fn unknown_trash_keeps_intent_and_saves_an_unknown_observation() {
    failed_helper(
        TrashMode::Unknown,
        EffectOutcome::Unknown,
        WorkOutcome::Unknown,
    );
}

fn failed_helper(mode: TrashMode, outcome: EffectOutcome, work: WorkOutcome) {
    let mut world = World::new();
    world.trash.mode = mode;
    let record = world.record("failed.m4a", b"owned failed audio");
    let failed = Retention {
        ledger: &world.ledger,
        stores: &world.stores,
        trash: &world.trash,
        clock: &FixedClock,
        sink: &mut world.sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap_err();
    assert!(matches!(failed.error, RetentionError::Trash(_)));
    assert!(failed.report.moved.is_empty());
    assert!(failed.report.completed.is_empty());
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
    assert!(
        world
            .ledger
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_none()
    );
    assert!(record.audio_path.exists());
    let view = world.ledger.recovery_view().unwrap();
    let effect = view
        .records
        .iter()
        .find(|saved| matches!(saved.fact, HistoryFact::EffectIntent { .. }))
        .unwrap()
        .id;
    assert!(view.records.iter().any(|saved| saved.fact
        == HistoryFact::EffectObserved {
            intent: effect,
            outcome
        }));
    assert!(view.records.iter().any(|saved| saved.fact
        == HistoryFact::WorkStep {
            recording: Some(record.id.clone()),
            outcome: work,
        }));
}

pub(super) fn reconciliation_helper_failure(
    mode: TrashMode,
    effect: EffectOutcome,
    work: WorkOutcome,
) {
    let mut world = World::new();
    world.trash.mode = mode;
    let record = world.record("reconcile-failed.m4a", b"owned reconciliation audio");
    let prepared = world.prepare(&record);
    let pending = world.ledger.pending_intents().unwrap();
    let run = world.run();
    let before = world.ledger.recovery_view().unwrap().records;
    let files = super::snapshot::tree(world.temp.path());
    let failed = reconcile_intents(
        run,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths,
        &mut world.sink,
    )
    .unwrap_err();
    let expected_error = match mode {
        TrashMode::Failed => TrashError::Failed("owned helper failed".into()),
        TrashMode::Unknown => TrashError::Unknown("owned helper outcome unknown".into()),
        TrashMode::Move => panic!("failure fixture needs a failed helper"),
    };
    assert_eq!(failed.error, RetentionError::Trash(expected_error));
    assert!(failed.progress.moved.is_empty());
    assert!(failed.progress.completed.is_empty());
    assert_eq!(world.ledger.pending_intents().unwrap(), pending);
    assert_eq!(
        world.ledger.by_id(&record.id).unwrap(),
        Some(record.clone())
    );
    assert_eq!(super::snapshot::tree(world.temp.path()), files);
    let trace = world.sink.trace.borrow();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0], Action::Trash(record.audio_path));
    let Action::Export(receipt) = &trace[1] else {
        panic!("failure facts must be exported after the helper result");
    };
    assert_eq!(receipt.run, run);
    assert!(receipt.completed.is_empty());
    assert_eq!(
        receipt
            .records
            .iter()
            .map(|saved| &saved.fact)
            .collect::<Vec<_>>(),
        vec![
            &HistoryFact::EffectObserved {
                intent: prepared.effect_id,
                outcome: effect
            },
            &HistoryFact::WorkStep {
                recording: Some(record.id),
                outcome: work
            },
        ]
    );
    assert!(
        receipt
            .records
            .iter()
            .all(|saved| saved.run == run && saved.occurred_at == NOW)
    );
    let mut expected = before;
    expected.extend(receipt.records.clone());
    assert_eq!(world.ledger.recovery_view().unwrap().records, expected);
}
