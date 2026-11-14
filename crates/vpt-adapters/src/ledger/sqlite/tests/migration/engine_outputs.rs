use crate::ledger::sqlite::tests::{database, tables, version};
use crate::ledger::sqlite::{SqliteLedger, map, migrations};
use vpt_application::ports::{EngineOutputLedger, LedgerError, RunHistory, TranscriptLedger};

#[test]
fn version_two_upgrade_preserves_transcript_and_history_on_rollback_and_success() {
    super::super::supervised("ledger_migration_preserve_v2");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    crate::ledger::contract::transcript::accepted_transcript_and_history_commit_together(&ledger);
    let accepted = crate::ledger::contract::transcript::fixture();
    let history = ledger.pending_exports().unwrap();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection.execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;").map_err(map)?;
        super::remove_tag_extension(connection)?;
        connection
            .execute_batch(
                "DROP TABLE known_term_publications; DROP TABLE owned_notes; DROP TABLE engine_outputs; PRAGMA user_version = 2;",
            )
            .map_err(map)
    })
    .unwrap();
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(
        observer
            .engine_output(std::path::Path::new("/unpublished.json"))
            .unwrap(),
        None
    );
    assert_eq!(observer.schema_version().unwrap(), 2);
    assert_eq!(
        observer.accepted_transcript(&accepted.recording).unwrap(),
        Some(accepted.clone())
    );
    drop(observer);
    database(root, |connection| {
        let injected = LedgerError::Io("precommit fixture".into());
        assert_eq!(
            migrations::migrate(connection, || Err(injected.clone())),
            Err(injected)
        );
        assert_eq!(version(connection), 2);
        assert!(!tables(connection).contains(&"engine_outputs".into()));
        Ok(())
    })
    .unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 19);
    assert_eq!(
        upgraded.accepted_transcript(&accepted.recording).unwrap(),
        Some(accepted)
    );
    assert_eq!(upgraded.pending_exports().unwrap(), history);
}
