use super::super::{OpenError, SCHEMA_VERSION, SqliteLedger};
use super::{database, map, seed, supervised, tables};
use crate::RootDir;
use std::os::unix::fs::PermissionsExt;
use vpt_application::ports::LedgerError;

#[test]
fn open_creates_a_private_current_database_in_write_ahead_mode() {
    supervised("ledger_schema_create");
}

#[test]
fn reopening_preserves_all_tables_and_repairs_database_and_companion_modes() {
    supervised("ledger_schema_reopen");
}

#[test]
fn writable_and_observational_open_preserve_future_schema_numbers() {
    supervised("ledger_schema_future");
}

#[test]
fn every_connection_enforces_foreign_keys() {
    supervised("ledger_schema_foreign_keys");
}

#[test]
fn failed_transactions_roll_back_and_successful_transactions_commit() {
    supervised("ledger_schema_transactions");
}

#[test]
fn nested_transactions_are_rejected_without_committing_outer_changes() {
    supervised("ledger_schema_nested");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    if scenario == "ledger_schema_future" {
        drop(seed(root));
        database(root, |c| {
            c.pragma_update(None, "user_version", 99).map_err(map)
        })
        .unwrap();
        let expected = OpenError::Ledger(LedgerError::UnsupportedSchema(99));
        assert_eq!(SqliteLedger::open(root).unwrap_err(), expected);
        assert_eq!(
            SqliteLedger::open_read_only(root.path()).unwrap_err(),
            expected
        );
        return;
    }
    let ledger = seed(root);
    assert_eq!(ledger.database_path(), root.path().join("vpt.db"));
    assert_eq!(ledger.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(SCHEMA_VERSION, 18);
    match scenario {
        "ledger_schema_create" | "ledger_schema_reopen" => {
            if scenario == "ledger_schema_reopen" {
                for leaf in ["vpt.db", "vpt.db-wal", "vpt.db-shm"] {
                    std::fs::set_permissions(
                        root.path().join(leaf),
                        std::fs::Permissions::from_mode(0o644),
                    )
                    .unwrap();
                }
                drop(seed(root));
            }
            ledger
                .read(|c| {
                    assert_eq!(
                        c.pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
                            .unwrap(),
                        "wal"
                    );
                    assert_eq!(
                        tables(c),
                        [
                            "action_sends",
                            "artifact_expirations",
                            "dirty_publications",
                            "engine_outputs",
                            "flags",
                            "history_export_failures",
                            "history_export_restarts",
                            "history_identity",
                            "history_records",
                            "history_runs",
                            "known_term_publications",
                            "learned_tags",
                            "mention_ambiguities",
                            "occasions",
                            "owned_briefs",
                            "owned_notes",
                            "proposals",
                            "recordings",
                            "relations",
                            "release_completions",
                            "release_payloads",
                            "releases",
                            "retention_intents",
                            "seen",
                            "tag_renames",
                            "tags",
                            "transcripts",
                            "verified_artifacts"
                        ]
                    );
                    Ok(())
                })
                .unwrap();
            for leaf in ["vpt.db", "vpt.db-wal", "vpt.db-shm"] {
                assert_eq!(
                    std::fs::metadata(root.path().join(leaf))
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o600,
                    "{leaf}"
                );
            }
        }
        "ledger_schema_foreign_keys" => {
            for _ in 0..2 {
                ledger
                    .read(|c| {
                        assert_eq!(
                            c.pragma_query_value(None, "foreign_keys", |row| row.get::<_, u32>(0))
                                .unwrap(),
                            1
                        );
                        Ok(())
                    })
                    .unwrap();
                let result = ledger.transaction(|t| {
                    t.execute(
                        "INSERT INTO tags (recording, tag, state, provenance) VALUES('absent', 'tag', 'confirmed', 'test')",
                        [],
                    )
                    .map(|_| ())
                    .map_err(map)
                });
                assert!(
                    matches!(result, Err(LedgerError::Conflict(_))),
                    "{result:?}"
                );
            }
            let observed = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
            observed
                .read(|c| {
                    assert_eq!(
                        c.pragma_query_value(None, "foreign_keys", |row| row.get::<_, u32>(0))
                            .unwrap(),
                        1
                    );
                    Ok(())
                })
                .unwrap();
        }
        "ledger_schema_transactions" | "ledger_schema_nested" => {
            let expected = LedgerError::Io("owned rollback".into());
            let result = ledger.transaction(|t| {
                t.execute(
                    "INSERT INTO dirty_publications VALUES('rolled-back', NULL, 'value', 1)",
                    [],
                )
                .map_err(map)?;
                if scenario == "ledger_schema_nested" {
                    let nested = rusqlite::Transaction::new_unchecked(
                        t,
                        rusqlite::TransactionBehavior::Immediate,
                    )
                    .unwrap_err();
                    assert!(matches!(map(nested), LedgerError::Io(_)));
                }
                Err::<(), _>(expected.clone())
            });
            assert_eq!(result, Err(expected));
            ledger
                .transaction(|t| {
                    t.execute(
                        "INSERT INTO dirty_publications VALUES('committed', NULL, 'value', 2)",
                        [],
                    )
                    .map(|_| ())
                    .map_err(map)
                })
                .unwrap();
            ledger
                .read(|c| {
                    assert_eq!(
                        c.query_row("SELECT target FROM dirty_publications", [], |row| row
                            .get::<_, String>(0))
                            .unwrap(),
                        "committed"
                    );
                    Ok(())
                })
                .unwrap();
        }
        _ => panic!("unknown schema scenario"),
    }
}
