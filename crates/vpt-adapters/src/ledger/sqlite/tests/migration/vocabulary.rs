use super::super::{database, supervised, version};
use crate::ledger::{
    contract,
    sqlite::{SqliteLedger, map, migrations},
};
use vpt_application::ports::*;

#[test]
fn version_six_upgrade_preserves_pending_terms_and_tag_state() {
    supervised("ledger_migration_preserve_v6");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    contract::tags::tags_and_private_provenance_commit_with_history(&ledger);
    contract::known_terms::known_terms_bytes_intent_and_history_commit_together(&ledger);
    let history = ledger.pending_exports().unwrap();
    let publications = ledger.pending_publications().unwrap();
    let recording = contract::tags::change().recording;
    let tags = ledger.tags(&recording).unwrap();
    let terms = contract::known_terms::publication();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection
            .execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;")
            .map_err(map)?;
        super::remove_vocabulary_extension(connection)?;
        connection
            .pragma_update(None, "user_version", 6)
            .map_err(map)
    })
    .unwrap();
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(observer.schema_version().unwrap(), 6);
    assert_eq!(
        observer
            .vocabulary_publication(&terms.entry.target)
            .unwrap(),
        Some(terms.clone())
    );
    assert_eq!(observer.tags(&recording).unwrap(), tags);
    drop(observer);
    database(root, |connection| {
        let error = LedgerError::Io("vocabulary migration precommit fixture".into());
        assert_eq!(migrations::migrate(connection, || Err(error.clone())), Err(error));
        assert_eq!(version(connection), 6);
        assert_eq!(connection.query_row("SELECT count(*) FROM pragma_table_info('known_term_publications') WHERE name='kind'", [], |row| row.get::<_, u32>(0)).unwrap(), 0);
        Ok(())
    }).unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 19);
    assert_eq!(
        upgraded
            .vocabulary_publication(&terms.entry.target)
            .unwrap(),
        Some(terms)
    );
    assert_eq!(upgraded.tags(&recording).unwrap(), tags);
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    assert_eq!(upgraded.pending_publications().unwrap(), publications);
}
