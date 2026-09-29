use super::support::*;
use vpt_application::ports::*;
use vpt_application::{KeptReason, ReconcileProgress, Retention, reconcile_intents};
use vpt_domain::history::EventDetail;
use vpt_domain::run::{HistoryFact, RunOutcome};

fn run(
    world: &mut World,
    dry: bool,
    initial: ReconcileProgress,
) -> Result<vpt_application::RetentionReport, Box<vpt_application::RetentionFailure>> {
    Retention {
        ledger: &world.ledger,
        stores: &world.stores,
        trash: &world.trash,
        clock: &FixedClock,
        sink: &mut world.sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(dry, initial)
}

#[test]
fn an_expired_clone_has_a_committed_exported_intent_before_the_move() {
    let mut world = World::new();
    let record = world.record("expired.m4a", b"expired owned audio");
    let report = run(&mut world, false, ReconcileProgress::default()).unwrap();
    assert_eq!(report.completed, vec![record.id.clone()]);
    assert_eq!(report.moved.len(), 1);
    assert_eq!(report.moved[0].path, record.audio_path);
    assert_eq!(
        std::fs::read(world.trash.destination.join("expired.m4a")).unwrap(),
        b"expired owned audio"
    );
    assert!(world.ledger.pending_intents().unwrap().is_empty());
    let trace = world.sink.trace.borrow();
    let prepared = trace.iter().position(|action| matches!(action, Action::Export(receipt)
        if receipt.records.iter().any(|record| matches!(record.fact, HistoryFact::EffectIntent { .. })))).unwrap();
    let moved = trace
        .iter()
        .position(|action| matches!(action, Action::Trash(_)))
        .unwrap();
    assert!(prepared < moved);
    assert_eq!(
        report_events(&world.sink.trace)[0].detail,
        EventDetail::Retention {
            completed: 1,
            failed: 0
        }
    );
    let Action::Export(final_receipt) = trace.last().unwrap() else {
        panic!("successful sweep must end with its committed report receipt");
    };
    assert!(final_receipt.completed.is_empty());
    assert_eq!(
        final_receipt
            .records
            .iter()
            .map(|saved| &saved.fact)
            .collect::<Vec<_>>(),
        vec![
            &HistoryFact::Event(report_events(&world.sink.trace)[0].clone()),
            &HistoryFact::RunFinished(RunOutcome::Completed),
        ]
    );
    assert!(
        final_receipt
            .records
            .iter()
            .all(|saved| saved.run == final_receipt.run && saved.occurred_at == NOW)
    );
    assert!(
        world
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .ends_with(&final_receipt.records)
    );
}

#[test]
fn an_untracked_file_survives_and_is_reported_kept() {
    let mut world = World::new();
    let path = world.paths.audio.join("untracked.m4a");
    std::fs::write(&path, b"untracked audio").unwrap();
    let report = run(&mut world, false, ReconcileProgress::default()).unwrap();
    assert!(report.moved.is_empty());
    assert_eq!(report.kept.len(), 1);
    assert_eq!(report.kept[0].path, path);
    assert_eq!(report.kept[0].reason, KeptReason::Untracked);
    assert_eq!(std::fs::read(path).unwrap(), b"untracked audio");
    assert!(report_events(&world.sink.trace).is_empty());
}

#[test]
fn audio_is_excluded_unless_include_audio_is_set() {
    let mut world = World::new();
    let record = world.record("excluded.m4a", b"excluded audio");
    world.settings.include_audio = false;
    let report = run(&mut world, false, ReconcileProgress::default()).unwrap();
    assert!(report.moved.is_empty());
    assert!(record.audio_path.exists());
    assert!(world.ledger.pending_intents().unwrap().is_empty());
}

#[test]
fn an_expired_clone_with_replaced_bytes_is_kept_without_an_intent() {
    let mut world = World::new();
    let record = world.record("changed.m4a", b"initial audio");
    std::fs::write(&record.audio_path, b"replacement audio").unwrap();
    let report = run(&mut world, false, ReconcileProgress::default()).unwrap();
    assert!(report.moved.is_empty());
    assert_eq!(report.kept[0].reason, KeptReason::DigestMismatch);
    assert!(world.ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        std::fs::read(record.audio_path).unwrap(),
        b"replacement audio"
    );
}

#[test]
fn retention_combines_recovery_and_new_moves_in_one_report_and_event() {
    let mut world = World::new();
    let recovered = world.record("recovered.m4a", b"recovered audio");
    world.prepare(&recovered);
    let absent = world.record("absent.m4a", b"already moved audio");
    world.prepare(&absent);
    std::fs::rename(
        &absent.audio_path,
        world.trash.destination.join("previous.m4a"),
    )
    .unwrap();
    let current = world.record("current.m4a", b"current audio");
    let run_id = world.run();
    let progress = reconcile_intents(
        run_id,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths.audio,
        &mut world.sink,
    )
    .unwrap();
    let report = run(&mut world, false, progress).unwrap();
    assert_eq!(report.completed, vec![recovered.id, absent.id, current.id]);
    assert_eq!(report.moved.len(), 2);
    let events = report_events(&world.sink.trace);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, vpt_domain::history::EventState::Done);
    assert_eq!(
        events[0].detail,
        EventDetail::Retention {
            completed: 2,
            failed: 0
        }
    );
}

#[test]
fn absent_reconciliation_does_not_invent_a_move_or_report_event() {
    let mut world = World::new();
    let record = world.record("absent.m4a", b"already moved audio");
    world.prepare(&record);
    std::fs::rename(
        &record.audio_path,
        world.trash.destination.join("previous.m4a"),
    )
    .unwrap();
    let run_id = world.run();
    let progress = reconcile_intents(
        run_id,
        &world.ledger,
        &world.stores,
        &world.trash,
        &FixedClock,
        &world.paths.audio,
        &mut world.sink,
    )
    .unwrap();
    let report = run(&mut world, false, progress).unwrap();
    assert_eq!(report.completed, vec![record.id]);
    assert!(report.moved.is_empty());
    assert!(report_events(&world.sink.trace).is_empty());
}

#[test]
fn dry_run_preserves_pending_intents_history_files_and_emits_no_event() {
    let mut world = World::new();
    let record = world.record("dry.m4a", b"dry audio");
    world.prepare(&record);
    let history = world.ledger.recovery_view().unwrap();
    let pending = world.ledger.pending_intents().unwrap();
    let files = super::snapshot::tree(world.temp.path());
    let report = run(&mut world, true, ReconcileProgress::default()).unwrap();
    assert_eq!(report.moved.len(), 1);
    assert!(report.completed.is_empty());
    assert_eq!(world.ledger.recovery_view().unwrap(), history);
    assert_eq!(world.ledger.pending_intents().unwrap(), pending);
    assert_eq!(super::snapshot::tree(world.temp.path()), files);
    assert!(world.sink.trace.borrow().is_empty());
}
