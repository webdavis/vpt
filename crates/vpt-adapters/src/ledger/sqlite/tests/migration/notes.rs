use crate::ledger::sqlite::tests::{database, tables, version};
use crate::ledger::sqlite::{SqliteLedger, map, migrations};
use vpt_application::ports::{EngineOutputLedger, LedgerError, NoteLedger, RunHistory};

#[test]
fn version_three_upgrade_preserves_outputs_history_and_pending_publication() {
    super::super::supervised("ledger_migration_preserve_v3");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    crate::ledger::contract::engine_outputs::raw_bytes_and_publication_commit_with_accepted_state(
        &ledger,
    );
    let output = crate::ledger::contract::engine_outputs::artifact();
    let history = ledger.pending_exports().unwrap();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection.execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;").map_err(map)?;
        super::remove_tag_extension(connection)?;
        connection
            .execute_batch("DROP TABLE known_term_publications; DROP TABLE owned_notes; PRAGMA user_version = 3;")
            .map_err(map)
    })
    .unwrap();
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(observer.schema_version().unwrap(), 3);
    assert_eq!(
        observer
            .note_path(
                &output.recording,
                vpt_application::ports::NoteStage::Transcript
            )
            .unwrap(),
        None
    );
    assert_eq!(
        observer
            .note_at(std::path::Path::new("/notes/missing.md"))
            .unwrap(),
        None
    );
    assert_eq!(
        observer.engine_output(&output.target).unwrap(),
        Some(output.clone())
    );
    drop(observer);
    database(root, |connection| {
        let error = LedgerError::Io("note migration precommit fixture".into());
        assert_eq!(
            migrations::migrate(connection, || Err(error.clone())),
            Err(error)
        );
        assert_eq!(version(connection), 3);
        assert!(!tables(connection).contains(&"owned_notes".to_owned()));
        Ok(())
    })
    .unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 18);
    assert_eq!(
        upgraded.engine_output(&output.target).unwrap(),
        Some(output)
    );
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    use vpt_application::ports::PublicationJournal;
    assert_eq!(
        upgraded.pending_publications().unwrap(),
        crate::ledger::contract::engine_outputs::delta()
            .commit
            .publications
    );
}
