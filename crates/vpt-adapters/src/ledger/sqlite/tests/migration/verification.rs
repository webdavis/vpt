use super::super::{database, supervised, version};
use crate::ledger::{
    contract::{engine_outputs::committed, proposals::fixture, verification},
    sqlite::{SqliteLedger, map, migrations},
};
use vpt_application::ports::*;

#[test]
fn version_eight_findings_require_rechecking_without_losing_saved_rows_or_history() {
    supervised("ledger_migration_preserve_v8");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    let run = fixture::seed(&ledger);
    let change = verification::change(&ledger);
    committed(verification::save(&ledger, run, change.clone()));
    let before = ledger.review_snapshot(&change.recording).unwrap().unwrap();
    let history = ledger.pending_exports().unwrap();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection
            .execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;")
            .map_err(map)?;
        connection
            .execute_batch("DROP TABLE verified_artifacts; PRAGMA user_version=8;")
            .map_err(map)
    })
    .unwrap();
    let bytes = crate::database::snapshot(root.path());
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let stale = observer
        .review_snapshot(&change.recording)
        .unwrap()
        .unwrap();
    assert_eq!(stale.stale_artifacts, vec![NoteStage::Transcript]);
    assert_eq!(stale.stale_findings.len(), 1);
    assert_eq!(
        stale.flags.len() + stale.stale_findings.len(),
        before.flags.len()
    );
    drop(observer);
    assert_eq!(crate::database::snapshot(root.path()), bytes);
    database(root, |connection| {
        let error = LedgerError::Io("injected migration failure".into());
        assert_eq!(
            migrations::migrate(connection, || Err(error.clone())),
            Err(error)
        );
        assert_eq!(version(connection), 8);
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='verified_artifacts'",
                    [],
                    |row| row.get::<_, u32>(0)
                )
                .unwrap(),
            0
        );
        Ok(())
    })
    .unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(
        upgraded
            .review_snapshot(&change.recording)
            .unwrap()
            .unwrap(),
        stale
    );
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    upgraded
        .read(|connection| {
            assert_eq!(
                connection
                    .query_row("SELECT open_flags FROM recordings", [], |row| row
                        .get::<_, u32>(0))
                    .unwrap(),
                1
            );
            Ok(())
        })
        .unwrap();
    committed(verification::save(&upgraded, run, change));
    assert_eq!(
        upgraded
            .review_snapshot(&before.accepted.unwrap().recording)
            .unwrap()
            .unwrap()
            .flags,
        before.flags
    );
}
