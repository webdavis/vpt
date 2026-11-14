use super::*;
use vpt_application::{ReconcileProgress, Retention, RetentionError};
mod boundaries;
mod fixture;
use fixture::*;

fn sweep(ledger: &impl Ledger) {
    let world = World::new();
    seed(ledger, &world);
    let before = ledger.retention_artifacts().unwrap();
    let trash = MovingTrash::new(ledger, &world);
    let mut sink = Sink;
    let report = Retention {
        ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(false, ReconcileProgress::default())
    .unwrap();
    assert_eq!(report.moved.len(), 6);
    assert_eq!(report.completed.len(), 3);
    assert_eq!(trash.moved.borrow().len(), 6);
    assert!(report.kept.is_empty());
    assert!(ledger.pending_intents().unwrap().is_empty());
    let after = ledger.retention_artifacts().unwrap();
    for artifact in before.iter().filter(|a| a.kind != ArtifactKind::Audio) {
        assert!(!artifact.path.exists());
        let expired = after.iter().find(|a| a.path == artifact.path).unwrap();
        assert_eq!(expired.status, RetentionStatus::Expired);
        assert_eq!(expired.trashed_at, Some(NOW));
        assert_eq!(expired.digest, artifact.digest);
        assert!(report.moved.iter().any(|row| row.path == artifact.path));
    }
}

fn dry_run(ledger: &impl Ledger) {
    let world = World::new();
    seed(ledger, &world);
    let before = (
        ledger.retention_artifacts().unwrap(),
        ledger.pending_exports().unwrap(),
    );
    let trash = MovingTrash::new(ledger, &world);
    let mut sink = Sink;
    let report = Retention {
        ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(true, ReconcileProgress::default())
    .unwrap();
    assert_eq!(report.moved.len(), 6);
    assert!(report.completed.is_empty());
    assert!(trash.moved.borrow().is_empty());
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        (
            ledger.retention_artifacts().unwrap(),
            ledger.pending_exports().unwrap()
        ),
        before
    );
    assert!(
        before
            .0
            .iter()
            .filter(|a| a.kind != ArtifactKind::Audio)
            .all(|a| a.path.exists())
    );
}

fn recovery(ledger: &impl Ledger, modified: bool) {
    let world = World::new();
    seed(ledger, &world);
    let run = engine::committed(ledger.begin_run(vpt_domain::run::RunKind::Retain, NOW)).run;
    let artifacts: Vec<_> = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .filter(|a| a.kind != ArtifactKind::Audio)
        .collect();
    for row in &artifacts {
        engine::committed(ledger.prepare_retention(
            run,
            &NewIntent {
                kind: row.kind,
                owner: row.owner.clone(),
                path: row.path.clone(),
                expected: world.stores.digest_of(&row.path).unwrap().unwrap(),
                recorded_at: NOW,
            },
        ));
    }
    if modified {
        std::fs::write(&artifacts[0].path, "replacement private bytes").unwrap();
    } else {
        std::fs::rename(&artifacts[0].path, world.trash.join("already-moved")).unwrap();
    }
    let trash = MovingTrash::new(ledger, &world);
    let mut sink = Sink;
    let result = Retention {
        ledger,
        stores: &world.stores,
        trash: &trash,
        clock: &FixedClock,
        sink: &mut sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .recover();
    if modified {
        let error = result.unwrap_err();
        assert_eq!(
            error.error,
            RetentionError::TargetModified(artifacts[0].path.clone())
        );
        assert_eq!(ledger.pending_intents().unwrap().len(), 6);
        assert!(trash.moved.borrow().is_empty());
        assert_eq!(
            std::fs::read_to_string(&artifacts[0].path).unwrap(),
            "replacement private bytes"
        );
    } else {
        let report = result.unwrap();
        assert_eq!(report.moved.len(), 5);
        assert_eq!(report.completed.len(), 3);
        assert!(ledger.pending_intents().unwrap().is_empty());
        assert_eq!(
            ledger
                .retention_artifacts()
                .unwrap()
                .iter()
                .filter(|a| a.trashed_at == Some(NOW))
                .count(),
            6
        );
    }
}

#[test]
fn memory_sweeps_all_later_stores_with_durable_intents() {
    sweep(&MemoryLedger::new());
}
#[test]
fn sqlite_sweeps_all_later_stores_with_durable_intents() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    sweep(&SqliteLedger::open(&root).unwrap());
}
#[test]
fn memory_later_store_dry_run_is_observational() {
    dry_run(&MemoryLedger::new());
}
#[test]
fn sqlite_later_store_dry_run_is_observational() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    dry_run(&SqliteLedger::open(&root).unwrap());
}
#[test]
fn memory_recovers_absent_and_unchanged_later_artifacts() {
    recovery(&MemoryLedger::new(), false);
}
#[test]
fn sqlite_recovers_absent_and_unchanged_later_artifacts() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    recovery(&SqliteLedger::open(&root).unwrap(), false);
}
#[test]
fn later_store_recovery_preserves_modified_content() {
    recovery(&MemoryLedger::new(), true);
}
