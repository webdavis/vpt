use super::support::*;
use vpt_application::ports::*;
use vpt_application::{RetentionError, reconcile_intents};
use vpt_domain::run::{EffectOutcome, HistoryFact, WorkOutcome};

#[test]
fn a_different_artifact_kind_cannot_move_or_expire_audio_during_recovery() {
    let mut world = World::new();
    let record = world.record("kind.m4a", b"owned audio");
    let run = world.run();
    let intent = NewIntent {
        kind: ArtifactKind::Transcript,
        owner: (record.id.clone()).into(),
        path: record.audio_path.clone(),
        expected: record.digest,
        recorded_at: NOW,
    };
    assert!(matches!(
        world.ledger.prepare_retention(run, &intent),
        HistoryWrite::Committed(_)
    ));
    let failure = reconcile_intents(
        run,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths,
        &mut world.sink,
    )
    .unwrap_err();
    assert!(matches!(failure.error, RetentionError::TargetModified(_)));
    assert_eq!(std::fs::read(&record.audio_path).unwrap(), b"owned audio");
    assert!(
        world
            .ledger
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_none()
    );
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
    assert!(
        !world
            .sink
            .trace
            .borrow()
            .iter()
            .any(|action| matches!(action, Action::Trash(_)))
    );
}

#[test]
fn failed_reconciliation_trash_keeps_intent_and_saves_failed_work_and_effect() {
    super::failures::reconciliation_helper_failure(
        TrashMode::Failed,
        EffectOutcome::Failed,
        WorkOutcome::Failed,
    );
}

#[test]
fn unknown_reconciliation_trash_keeps_intent_and_saves_unknown_work_and_effect() {
    super::failures::reconciliation_helper_failure(
        TrashMode::Unknown,
        EffectOutcome::Unknown,
        WorkOutcome::Unknown,
    );
}

#[test]
fn an_absent_path_completes_the_expiration_without_the_trash() {
    let mut world = World::new();
    let record = world.record("absent.m4a", b"owned absent audio");
    let prepared = world.prepare(&record);
    std::fs::rename(
        &record.audio_path,
        world.trash.destination.join("already-moved.m4a"),
    )
    .unwrap();
    let run = world.run();
    let progress = reconcile_intents(
        run,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths,
        &mut world.sink,
    )
    .unwrap();
    assert!(progress.moved.is_empty());
    assert_eq!(progress.completed, vec![(record.id.clone()).into()]);
    assert!(world.ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        world
            .ledger
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at,
        Some(NOW)
    );
    assert!(
        !world
            .sink
            .trace
            .borrow()
            .iter()
            .any(|action| matches!(action, Action::Trash(_)))
    );
    assert!(
        world
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|saved| saved.run == run
                && saved.fact
                    == HistoryFact::EffectObserved {
                        intent: prepared.effect_id,
                        outcome: EffectOutcome::Completed,
                    })
    );
}

#[test]
fn an_unchanged_original_is_moved_again_and_completed() {
    let mut world = World::new();
    let record = world.record("original.m4a", b"owned original audio");
    world.prepare(&record);
    let run = world.run();
    let progress = reconcile_intents(
        run,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths,
        &mut world.sink,
    )
    .unwrap();
    assert_eq!(progress.completed, vec![(record.id.clone()).into()]);
    assert_eq!(progress.moved.len(), 1);
    assert_eq!(progress.moved[0].owner, (record.id).into());
    assert!(!record.audio_path.exists());
    assert_eq!(
        std::fs::read(world.trash.destination.join("original.m4a")).unwrap(),
        b"owned original audio"
    );
    assert!(world.ledger.pending_intents().unwrap().is_empty());
}

#[test]
fn replaced_content_is_a_refusal_naming_the_path_and_stays_pending() {
    let mut world = World::new();
    let record = world.record("replaced.m4a", b"owned original audio");
    world.prepare(&record);
    std::fs::write(&record.audio_path, b"owned replacement").unwrap();
    let run = world.run();
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
    assert_eq!(
        failed.error,
        RetentionError::TargetModified(record.audio_path.clone())
    );
    assert!(failed.progress.moved.is_empty());
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
    assert_eq!(
        std::fs::read(&record.audio_path).unwrap(),
        b"owned replacement"
    );
    assert!(
        !world
            .sink
            .trace
            .borrow()
            .iter()
            .any(|action| matches!(action, Action::Trash(_)))
    );
}

#[test]
fn a_later_refusal_retains_the_first_move_and_its_recording_identity() {
    let mut world = World::new();
    let first = world.record("first.m4a", b"first original");
    let second = world.record("second.m4a", b"second original");
    world.prepare(&first);
    world.prepare(&second);
    std::fs::write(&second.audio_path, b"changed").unwrap();
    let run = world.run();
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
    assert_eq!(
        failed.error,
        RetentionError::TargetModified(second.audio_path)
    );
    assert_eq!(failed.progress.completed, vec![(first.id.clone()).into()]);
    assert_eq!(failed.progress.moved[0].owner, (first.id).into());
    assert_eq!(
        std::fs::read(world.trash.destination.join("first.m4a")).unwrap(),
        b"first original"
    );
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
}

#[test]
fn an_old_unmatched_journal_is_observed_without_inventing_an_original_effect() {
    let mut world = World::new();
    let record = world.record("legacy.m4a", b"old unmatched audio");
    let id = world.ledger.record_intent(&new_intent(&record)).unwrap();
    let run = world.run();
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
    assert_eq!(
        failed.error,
        RetentionError::MissingHistory { intent_id: id }
    );
    assert!(failed.progress.moved.is_empty());
    let records = world.ledger.recovery_view().unwrap().records;
    assert!(records.iter().any(
        |record| matches!(&record.fact, HistoryFact::MissingRetentionHistory { intent_id }
        if *intent_id == u64::try_from(id).unwrap())
    ));
    assert!(
        !records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::EffectIntent { .. }))
    );
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
    assert!(record.audio_path.exists());
}

#[test]
fn linked_intent_target_never_reaches_trash_or_referent_content() {
    let mut world = World::new();
    let record = world.record("linked.m4a", b"owned original");
    world.prepare(&record);
    let held = world.paths.audio.join("held.m4a");
    std::fs::rename(&record.audio_path, &held).unwrap();
    std::os::unix::fs::symlink(&held, &record.audio_path).unwrap();
    let run = world.run();
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
    assert!(matches!(
        failed.error,
        RetentionError::Stores(StoreError::Escape(_))
    ));
    assert_eq!(std::fs::read(held).unwrap(), b"owned original");
    assert!(
        !world
            .sink
            .trace
            .borrow()
            .iter()
            .any(|action| matches!(action, Action::Trash(_)))
    );
}
