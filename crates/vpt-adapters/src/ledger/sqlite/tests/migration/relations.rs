use super::super::{database, supervised, version};
use crate::ledger::{
    contract,
    sqlite::{SqliteLedger, map, migrations},
};
use vpt_application::ports::*;

#[test]
fn version_seven_upgrade_preserves_relations_and_history_with_atomic_rollback() {
    supervised("ledger_migration_preserve_v7");
}
pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    contract::relations::relation_state_commits_with_history_and_private_provenance(&ledger);
    let history = ledger.pending_exports().unwrap();
    let id = contract::relations::delta().relations.unwrap().recording;
    let relations = ledger.relations(&id).unwrap();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection
            .execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;")
            .map_err(map)?;
        super::remove_relation_extension(connection)?;
        connection
            .pragma_update(None, "user_version", 7)
            .map_err(map)
    })
    .unwrap();
    let before = crate::database::snapshot(root.path());
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert!(observer.mention_ambiguities(&id).unwrap().is_empty());
    assert_eq!(observer.relations(&id).unwrap(), relations);
    drop(observer);
    assert_eq!(crate::database::snapshot(root.path()), before);
    database(root, |connection| {
        let error = LedgerError::Io("relation migration fixture".into());
        assert_eq!(migrations::migrate(connection, || Err(error.clone())), Err(error));
        assert_eq!(version(connection), 7);
        assert_eq!(connection.query_row("SELECT count(*) FROM sqlite_schema WHERE name IN ('mention_ambiguities','recordings_capture_order')", [], |r| r.get::<_, u32>(0)).unwrap(), 0);
        Ok(())
    }).unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 19);
    assert_eq!(upgraded.relations(&id).unwrap(), relations);
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    assert!(upgraded.mention_ambiguities(&id).unwrap().is_empty());
}
