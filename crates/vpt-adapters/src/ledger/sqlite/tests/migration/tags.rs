use super::super::{database, supervised, version};
use crate::ledger::{
    contract,
    sqlite::{SqliteLedger, map, migrations},
};
use vpt_application::ports::*;
use vpt_domain::tags::{Provenance, State, Tag};

#[test]
fn version_five_upgrade_preserves_tag_rows_notes_pending_terms_and_history() {
    supervised("ledger_migration_preserve_v5");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    contract::notes::note_bytes_paths_and_history_commit_together(&ledger);
    let HistoryWrite::Committed(run) = ledger.begin_run(
        vpt_domain::run::RunKind::Publish,
        vpt_domain::time::UtcInstant { secs: 20 },
    ) else {
        panic!("run");
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &contract::known_terms::delta(),
            &contract::known_terms::facts()
        ),
        HistoryWrite::Committed(_)
    ));
    let history = ledger.pending_exports().unwrap();
    let publications = ledger.pending_publications().unwrap();
    let note = contract::notes::artifact();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection.execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;").map_err(map)?;
        super::remove_tag_extension(connection)?;
        connection.execute("INSERT INTO tags (recording, tag, state, provenance) VALUES (?1, 'zeta', 'confirmed', '{\"kind\":\"manual\"}'), (?1, 'alpha', 'rejected', '{\"kind\":\"manual\"}')", [note.recording.as_str()]).map_err(map)?;
        connection.pragma_update(None, "user_version", 5).map_err(map)
    }).unwrap();
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(observer.tags(&note.recording).unwrap(), None);
    assert_eq!(observer.schema_version().unwrap(), 5);
    drop(observer);
    database(root, |connection| {
        let error = LedgerError::Io("tag migration precommit fixture".into());
        assert_eq!(
            migrations::migrate(connection, || Err(error.clone())),
            Err(error)
        );
        assert_eq!(version(connection), 5);
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM pragma_table_info('tags') WHERE name='ordinal'",
                    [],
                    |row| row.get::<_, u32>(0)
                )
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM tags", [], |row| row.get::<_, u32>(0))
                .unwrap(),
            2
        );
        Ok(())
    })
    .unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 19);
    assert_eq!(
        upgraded.tags(&note.recording).unwrap(),
        Some(vec![
            Tag {
                name: "alpha".into(),
                state: State::Rejected,
                provenance: Provenance::Manual,
                updated_at: None
            },
            Tag {
                name: "zeta".into(),
                state: State::Confirmed,
                provenance: Provenance::Manual,
                updated_at: None
            },
        ])
    );
    assert_eq!(upgraded.note_at(&note.target).unwrap(), Some(note));
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    assert_eq!(upgraded.pending_publications().unwrap(), publications);
    let terms = contract::known_terms::publication();
    assert_eq!(
        upgraded
            .vocabulary_publication(&terms.entry.target)
            .unwrap(),
        Some(terms)
    );
}
