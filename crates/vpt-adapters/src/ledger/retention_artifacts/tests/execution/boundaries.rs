use super::*;
use std::path::{Path, PathBuf};
use vpt_domain::{layout::StoreKey, retention::Hold};

#[test]
fn each_store_uses_its_own_hold_and_preserves_modified_pending_and_untracked_notes() {
    let ledger = MemoryLedger::new();
    let mut world = World::new();
    seed(&ledger, &world);
    world.settings.holds = vec![(StoreKey::Analysis, Hold::of_seconds(1))];
    let rows = ledger.retention_artifacts().unwrap();
    let analysis = rows
        .iter()
        .find(|a| a.kind == ArtifactKind::Analysis)
        .unwrap();
    let mut changed = std::fs::read_to_string(&analysis.path)
        .unwrap()
        .replace("vpt-stage: analysis", "vpt-stage: transcript");
    std::fs::write(&analysis.path, &changed).unwrap();
    std::fs::write(world.paths.analysis.join("manual.md"), "operator file").unwrap();
    let trash = MovingTrash::new(&ledger, &world);
    let mut sink = Sink;
    let report = Retention {
        ledger: &ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap();
    assert!(report.moved.is_empty());
    assert!(
        report
            .kept
            .iter()
            .any(|r| r.path == analysis.path && r.reason.as_str() == "ownership_mismatch")
    );
    assert!(report.kept.iter().any(|r| r.reason.as_str() == "untracked"));
    assert!(ledger.pending_intents().unwrap().is_empty());
    changed = changed.replace("vpt-stage: transcript", "vpt-stage: analysis");
    std::fs::write(&analysis.path, &changed).unwrap();
    ledger
        .commit(&LedgerCommit {
            publications: vec![DirtyPublication {
                target: analysis.path.clone(),
                expected_previous: Some(analysis.digest),
                intended: analysis.digest,
                recorded_at: NOW,
            }],
            ..Default::default()
        })
        .unwrap();
    let report = Retention {
        ledger: &ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap();
    assert!(report.moved.is_empty());
    assert!(
        report
            .kept
            .iter()
            .any(|r| r.path == analysis.path && r.reason.as_str() == "publication_pending")
    );
    ledger.clear_publication(&analysis.path).unwrap();
    let report = Retention {
        ledger: &ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap();
    assert_eq!(report.moved.len(), 1);
    assert_eq!(report.moved[0].path, analysis.path);
    assert_eq!(report.moved[0].store, StoreKey::Analysis);
    assert!(
        rows.iter()
            .filter(|a| a.kind != ArtifactKind::Analysis && a.kind != ArtifactKind::Audio)
            .all(|a| a.path.exists())
    );
}

struct ReplaceAfterIntent;
impl EventSink for ReplaceAfterIntent {
    fn after_commit(&mut self, receipt: &HistoryReceipt) -> SinkOutcome {
        for row in &receipt.records {
            if let vpt_domain::run::HistoryFact::EffectIntent {
                reference: vpt_domain::run::EffectReference::RetentionIntent { path, .. },
                ..
            } = &row.fact
            {
                std::fs::write(path, "operator replacement after journal commit").unwrap();
            }
        }
        SinkOutcome::Continue {
            warnings: Default::default(),
        }
    }
    fn recover(&mut self, _: &vpt_domain::run::RecoveryView) -> SinkOutcome {
        panic!("unexpected replay")
    }
}
#[test]
fn a_note_changed_after_intent_is_preserved_and_its_intent_remains_pending() {
    let ledger = MemoryLedger::new();
    let mut world = World::new();
    seed(&ledger, &world);
    world.settings.holds = vec![(StoreKey::Transcripts, Hold::of_seconds(1))];
    let trash = MovingTrash::new(&ledger, &world);
    let mut sink = ReplaceAfterIntent;
    let failure = Retention {
        ledger: &ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap_err();
    assert!(matches!(failure.error, RetentionError::TargetModified(_)));
    assert!(trash.moved.borrow().is_empty());
    assert_eq!(ledger.pending_intents().unwrap().len(), 1);
    assert_eq!(
        std::fs::read_to_string(world.paths.transcripts.join("owned.md")).unwrap(),
        "operator replacement after journal commit"
    );
}
struct UnknownAfterMove<'a>(MovingTrash<'a, MemoryLedger>);
impl Trash for UnknownAfterMove<'_> {
    fn trash(&self, path: &Path) -> Result<PathBuf, TrashError> {
        self.0.trash(path)?;
        Err(TrashError::Unknown("lost reply".into()))
    }
    fn take_diagnostics(&self) -> Vec<String> {
        vec![]
    }
}
#[test]
fn an_unknown_brief_move_recovers_as_an_occasion_without_moving_twice() {
    let ledger = MemoryLedger::new();
    let mut world = World::new();
    seed(&ledger, &world);
    world.settings.holds = vec![(StoreKey::Briefs, Hold::of_seconds(1))];
    let brief = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == ArtifactKind::Brief)
        .unwrap();
    let trash = UnknownAfterMove(MovingTrash::new(&ledger, &world));
    let mut sink = Sink;
    let failed = Retention {
        ledger: &ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap_err();
    assert!(matches!(
        failed.error,
        RetentionError::Trash(TrashError::Unknown(_))
    ));
    assert_eq!(ledger.pending_intents().unwrap().len(), 1);
    assert!(!brief.path.exists());
    let report = Retention {
        ledger: &ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .recover()
    .unwrap();
    assert_eq!(report.completed, vec![brief.owner]);
    assert!(report.moved.is_empty());
    assert_eq!(trash.0.moved.borrow().len(), 1);
    assert!(ledger.pending_intents().unwrap().is_empty());
}
