use crate::ledger::sqlite::tests::{database, tables, version};
use crate::ledger::sqlite::{SqliteLedger, map, migrations};
use vpt_application::ports::{
    LedgerError, NoteLedger, PublicationJournal, RunHistory, VocabularyLedger,
};

#[test]
fn version_four_upgrade_preserves_notes_and_history_and_rolls_back_on_failure() {
    super::super::supervised("ledger_migration_preserve_v4");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    crate::ledger::contract::notes::note_bytes_paths_and_history_commit_together(&ledger);
    let note = crate::ledger::contract::notes::artifact();
    let history = ledger.pending_exports().unwrap();
    let publications = ledger.pending_publications().unwrap();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection
            .execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;")
            .map_err(map)?;
        super::remove_tag_extension(connection)?;
        connection
            .execute_batch("DROP TABLE known_term_publications; PRAGMA user_version=4;")
            .map_err(map)
    })
    .unwrap();
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(
        observer
            .vocabulary_publication(std::path::Path::new("/config/terms"))
            .unwrap(),
        None
    );
    assert_eq!(observer.schema_version().unwrap(), 4);
    assert_eq!(observer.note_at(&note.target).unwrap(), Some(note.clone()));
    drop(observer);
    database(root, |connection| {
        let error = LedgerError::Io("known-term migration precommit fixture".into());
        assert_eq!(
            migrations::migrate(connection, || Err(error.clone())),
            Err(error)
        );
        assert_eq!(version(connection), 4);
        assert!(!tables(connection).contains(&"known_term_publications".to_owned()));
        Ok(())
    })
    .unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 16);
    assert_eq!(upgraded.note_at(&note.target).unwrap(), Some(note));
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    assert_eq!(upgraded.pending_publications().unwrap(), publications);
}
