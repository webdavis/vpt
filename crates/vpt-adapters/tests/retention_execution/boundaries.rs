use super::support::*;
use std::path::PathBuf;
use vpt_application::ports::*;
use vpt_application::{ReconcileProgress, Retention, RetentionError, reconcile_intents};
use vpt_domain::history::{EventDetail, EventState};
use vpt_domain::run::{HistoryFact, RunOutcome};

#[test]
fn disabled_retention_refuses_without_history_or_filesystem_effects() {
    let mut world = World::new();
    world.record("disabled.m4a", b"disabled audio");
    world.settings.enabled = false;
    let before = super::snapshot::tree(world.temp.path());
    let history = world.ledger.recovery_view().unwrap();
    let failed = run(&mut world).unwrap_err();
    assert_eq!(failed.error, RetentionError::Disabled);
    assert_eq!(world.ledger.recovery_view().unwrap(), history);
    assert_eq!(super::snapshot::tree(world.temp.path()), before);
    assert!(world.sink.trace.borrow().is_empty());
}

#[test]
fn a_never_hold_keeps_owned_audio_without_a_retention_intent() {
    let mut world = World::new();
    let record = world.record("never.m4a", b"never audio");
    world.settings.holds.clear();
    assert!(run(&mut world).unwrap().moved.is_empty());
    assert!(world.ledger.pending_intents().unwrap().is_empty());
    assert_eq!(std::fs::read(record.audio_path).unwrap(), b"never audio");
}

#[test]
fn an_unexpired_clone_keeps_owned_audio_without_a_retention_intent() {
    let mut world = World::new();
    let record = world.record("future.m4a", b"future audio");
    let file = std::fs::File::options()
        .write(true)
        .open(&record.audio_path)
        .unwrap();
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(NOW.secs as u64 + 1))
        .unwrap();
    drop(file);
    assert!(run(&mut world).unwrap().moved.is_empty());
    assert!(world.ledger.pending_intents().unwrap().is_empty());
    assert_eq!(std::fs::read(record.audio_path).unwrap(), b"future audio");
}

#[test]
fn a_configured_non_audio_store_is_kept_as_untracked_in_this_stage() {
    let mut world = World::new();
    world.settings.holds.push((
        vpt_domain::layout::StoreKey::Transcripts,
        vpt_domain::retention::Hold::of_seconds(1),
    ));
    let path = world.paths.transcripts.join("owned-note.md");
    std::fs::write(&path, b"owned note").unwrap();
    let report = run(&mut world).unwrap();
    assert!(report.moved.is_empty());
    assert_eq!(
        report.kept,
        vec![vpt_application::Kept {
            path: path.clone(),
            reason: vpt_application::KeptReason::Untracked
        }]
    );
    assert_eq!(std::fs::read(path).unwrap(), b"owned note");
    assert!(world.ledger.pending_intents().unwrap().is_empty());
}

enum Replacement {
    Link,
    Root,
    Bytes,
}

struct ReplacingSink<'a> {
    inner: &'a mut Sink,
    path: PathBuf,
    replacement: Option<Replacement>,
}

impl EventSink for ReplacingSink<'_> {
    fn after_commit(&mut self, receipt: &HistoryReceipt) -> SinkOutcome {
        let outcome = self.inner.after_commit(receipt);
        if receipt
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::EffectIntent { .. }))
        {
            match self.replacement.take() {
                Some(Replacement::Link) => {
                    let held = self.path.with_extension("held");
                    std::fs::rename(&self.path, &held).unwrap();
                    std::os::unix::fs::symlink(held, &self.path).unwrap();
                }
                Some(Replacement::Root) => {
                    let root = self.path.parent().unwrap();
                    std::fs::rename(root, root.with_extension("held")).unwrap();
                    std::fs::create_dir(root).unwrap();
                    std::fs::write(&self.path, b"replacement audio").unwrap();
                }
                Some(Replacement::Bytes) => {
                    std::fs::write(&self.path, b"replacement audio").unwrap()
                }
                None => {}
            }
        }
        outcome
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        panic!("no recovery export");
    }
}

#[test]
fn a_link_replaced_after_preparation_export_never_reaches_trash() {
    replacement(Replacement::Link);
}

#[test]
fn a_root_replaced_after_preparation_export_never_reaches_trash() {
    replacement(Replacement::Root);
}

#[test]
fn bytes_replaced_after_preparation_export_never_reach_trash() {
    replacement(Replacement::Bytes);
}

fn replacement(change: Replacement) {
    let mut world = World::new();
    let record = world.record("owned.m4a", b"initial owned audio");
    let changed_bytes = matches!(change, Replacement::Bytes);
    let failed = Retention {
        ledger: &world.ledger,
        stores: &world.stores,
        trash: &world.trash,
        clock: &FixedClock,
        sink: &mut ReplacingSink {
            inner: &mut world.sink,
            path: record.audio_path.clone(),
            replacement: Some(change),
        },
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap_err();
    if changed_bytes {
        assert_eq!(
            failed.error,
            RetentionError::TargetModified(record.audio_path.clone())
        );
        assert_eq!(
            std::fs::read(&record.audio_path).unwrap(),
            b"replacement audio"
        );
    } else {
        assert!(matches!(
            failed.error,
            RetentionError::Stores(StoreError::Escape(_))
        ));
    }
    assert!(failed.report.moved.is_empty());
    assert_eq!(world.ledger.pending_intents().unwrap().len(), 1);
    assert!(
        !world
            .sink
            .trace
            .borrow()
            .iter()
            .any(|action| matches!(action, Action::Trash(_)))
    );
    assert!(
        std::fs::read_dir(&world.trash.destination)
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn later_helper_failure_retains_recovered_moves_in_one_safe_report_event() {
    let mut world = World::new();
    let recovered = world.record("recovered.m4a", b"recovered audio");
    world.prepare(&recovered);
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
    world.record("failed.m4a", b"failed audio");
    world.trash.mode = TrashMode::Failed;
    let failed = Retention {
        ledger: &world.ledger,
        stores: &world.stores,
        trash: &world.trash,
        clock: &FixedClock,
        sink: &mut world.sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, progress)
    .unwrap_err();
    assert!(matches!(failed.error, RetentionError::Trash(_)));
    assert_eq!(failed.report.completed, vec![recovered.id]);
    assert_eq!(failed.report.moved.len(), 1);
    let events = report_events(&world.sink.trace);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, EventState::Failed);
    assert_eq!(
        events[0].detail,
        EventDetail::Retention {
            completed: 1,
            failed: 1
        }
    );
    assert!(events[0].recording.is_none());
    assert!(events[0].paths.is_empty());
    assert!(
        world
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| record.fact == HistoryFact::RunFinished(RunOutcome::Failed))
    );
}

struct WarningSink;
impl EventSink for WarningSink {
    fn after_commit(&mut self, _: &HistoryReceipt) -> SinkOutcome {
        SinkOutcome::Continue {
            warnings: [SinkWarning::ExportUnavailable].into(),
        }
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        panic!("no recovery export");
    }
}

struct DiagnosticTrash<'a>(&'a MovingTrash);
impl Trash for DiagnosticTrash<'_> {
    fn trash(&self, path: &std::path::Path) -> Result<PathBuf, TrashError> {
        self.0.trash(path)
    }
    fn take_diagnostics(&self) -> Vec<String> {
        vec!["bounded helper diagnostic".into()]
    }
}

#[test]
fn optional_export_warnings_and_helper_diagnostics_preserve_successful_moves() {
    let world = World::new();
    world.record("warnings.m4a", b"warning audio");
    let report = Retention {
        ledger: &world.ledger,
        stores: &world.stores,
        trash: &DiagnosticTrash(&world.trash),
        clock: &FixedClock,
        sink: &mut WarningSink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap();
    assert_eq!(report.moved.len(), 1);
    assert_eq!(report.warnings, [SinkWarning::ExportUnavailable].into());
    assert_eq!(report.diagnostics, vec!["bounded helper diagnostic"]);
}

fn run(
    world: &mut World,
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
    .run(false, ReconcileProgress::default())
}
