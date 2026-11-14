mod actions;
mod claims;
mod engine_outputs;
mod expirations;
mod known_terms;
mod learned;
mod notes;
mod relations;
mod renames;
mod tags;
mod verification;
mod vocabulary;
mod window;
use super::super::{error, migrations};
use super::{database, supervised, tables, version};
use crate::RootDir;
use rusqlite::{Transaction, TransactionBehavior};
use std::cell::Cell;
use vpt_application::ports::LedgerError;

#[test]
fn version_one_schema_is_created_in_an_immediate_transaction() {
    supervised("ledger_migration_create");
}

#[test]
fn migration_failure_rolls_back_the_schema_and_version() {
    supervised("ledger_migration_rollback");
}

#[test]
fn future_schema_is_refused_without_running_the_migration() {
    supervised("ledger_migration_future");
}

#[test]
fn migration_root_replacement_rolls_back_before_commit() {
    supervised("ledger_migration_root_replacement");
}

#[test]
fn version_one_rows_and_pending_intents_survive_history_migration() {
    supervised("ledger_migration_preserve_v1");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    if scenario == "ledger_migration_preserve_v10" {
        window::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v9" {
        claims::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v8" {
        verification::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v7" {
        relations::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v6" {
        vocabulary::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v5" {
        tags::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v4" {
        known_terms::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v3" {
        notes::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v2" {
        engine_outputs::run(root);
        return;
    }
    if scenario == "ledger_migration_preserve_v1" {
        database(root, |connection| {
            let fixture = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)
                .map_err(super::map)?;
            fixture.execute_batch(migrations::SCHEMA_V1).map_err(super::map)?;
            fixture.pragma_update(None, "user_version", 1).map_err(super::map)?;
            fixture.execute(
                "INSERT INTO dirty_publications VALUES ('/h/pending.md', NULL, 'digest', 7)",
                [],
            ).map_err(super::map)?;
            fixture.execute(
                "INSERT INTO retention_intents (artifact_kind, recording, path, expected, recorded_at) VALUES ('audio', NULL, '/h/audio/pending.m4a', 'digest', 8)",
                [],
            ).map_err(super::map)?;
            fixture.execute(
                "INSERT INTO seen (path, file_name, size, mtime_secs, mtime_nanos, flags, first_seen, last_seen, deferral_count) VALUES ('/vm/Recordings/v1.m4a', 'v1.m4a', 8, 7, 0, 0, 7, 8, 0)",
                [],
            ).map_err(super::map)?;
            let recording = crate::ledger::contract::record(94);
            fixture.execute(
                "INSERT INTO recordings (id, digest, captured_at, captured_offset, duration_secs, title_source, ingested_at, audio_path, stage_transcribe, stage_note, stage_analysis) VALUES (?1, ?2, ?3, ?4, ?5, 'unavailable', ?6, ?7, 'pending', 'pending', 'pending')",
                rusqlite::params![recording.id.as_str(), recording.digest.hex(), recording.captured_at.secs, recording.captured_offset.secs, i64::try_from(recording.duration_secs).unwrap(), recording.ingested_at.secs, recording.audio_path.to_string_lossy()],
            ).map_err(super::map)?;
            fixture.commit().map_err(super::map)?;
            assert!(connection.is_autocommit());
            assert_eq!(version(connection), 1);
            migrations::migrate(connection, || Ok(()))?;
            assert_eq!(version(connection), 18);
            assert_eq!(connection.query_row("SELECT count(*) FROM dirty_publications", [], |row| row.get::<_, u32>(0)).unwrap(), 1);
            assert_eq!(connection.query_row("SELECT count(*) FROM retention_intents WHERE completed_at IS NULL", [], |row| row.get::<_, u32>(0)).unwrap(), 1);
            assert_eq!(connection.query_row("SELECT count(*) FROM seen WHERE path = '/vm/Recordings/v1.m4a'", [], |row| row.get::<_, u32>(0)).unwrap(), 1);
            assert_eq!(connection.query_row("SELECT count(*) FROM recordings WHERE id = ?1", [recording.id.as_str()], |row| row.get::<_, u32>(0)).unwrap(), 1);
            assert_eq!(connection.query_row("SELECT length(namespace) FROM history_identity WHERE id = 1", [], |row| row.get::<_, u32>(0)).unwrap(), 16);
            Ok(())
        }).unwrap();
        return;
    }
    if scenario == "ledger_migration_root_replacement" {
        let expected = LedgerError::PathEscape(root.path().into());
        let result = database(root, |connection| {
            let entered = Cell::new(false);
            let result = migrations::migrate(connection, || {
                entered.set(true);
                std::fs::rename(root.path(), root.path().with_file_name("held")).unwrap();
                std::fs::create_dir(root.path()).unwrap();
                std::fs::write(root.path().join("sentinel"), b"unchanged").unwrap();
                root.revalidate().map_err(error::contained)
            });
            assert!(entered.get(), "migration must validate before commit");
            assert_eq!(result, Err(expected.clone()));
            assert!(connection.is_autocommit());
            assert_eq!(version(connection), 0);
            assert!(tables(connection).is_empty());
            result
        });
        assert_eq!(result, Err(expected));
        assert_eq!(
            std::fs::read(root.path().join("sentinel")).unwrap(),
            b"unchanged"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        return;
    }
    database(root, |connection| {
        if scenario == "ledger_migration_future" {
            connection.pragma_update(None, "user_version", 99).unwrap();
            assert_eq!(
                migrations::current(connection),
                Err(LedgerError::UnsupportedSchema(99))
            );
            assert_eq!(
                migrations::migrate(connection, || panic!("future schema must not commit")),
                Err(LedgerError::UnsupportedSchema(99))
            );
            assert!(tables(connection).is_empty());
            assert_eq!(version(connection), 99);
        } else {
            let entered = Cell::new(false);
            let injected = LedgerError::Io("owned precommit failure".into());
            let result = migrations::migrate(connection, || {
                entered.set(true);
                assert!(!connection.is_autocommit());
                assert_eq!(
                    // SAFETY: the borrowed connection retains this live SQLite handle.
                    unsafe {
                        rusqlite::ffi::sqlite3_txn_state(connection.handle(), std::ptr::null())
                    },
                    2
                );
                assert_eq!(version(connection), 18);
                assert_eq!(tables(connection).len(), 28);
                if scenario == "ledger_migration_rollback" {
                    Err(injected.clone())
                } else {
                    Ok(())
                }
            });
            assert!(
                entered.get(),
                "migration must reach precommit with its schema transaction"
            );
            assert!(connection.is_autocommit());
            if scenario == "ledger_migration_rollback" {
                assert_eq!(result, Err(injected));
                assert!(tables(connection).is_empty());
                assert_eq!(version(connection), 0);
            } else {
                result.unwrap();
                assert_eq!(migrations::current(connection).unwrap(), 18);
                assert_eq!(tables(connection).len(), 28);
                migrations::migrate(connection, || Ok(())).unwrap();
                assert_eq!(tables(connection).len(), 28);
            }
        }
        Ok(())
    })
    .unwrap();
}

fn remove_tag_extension(connection: &rusqlite::Connection) -> Result<(), LedgerError> {
    remove_vocabulary_extension(connection)?;
    connection.execute_batch("DROP INDEX tags_order; ALTER TABLE tags DROP COLUMN ordinal; ALTER TABLE tags DROP COLUMN updated_at; ALTER TABLE recordings DROP COLUMN tags_initialized;").map_err(super::map)
}

fn remove_vocabulary_extension(connection: &rusqlite::Connection) -> Result<(), LedgerError> {
    remove_relation_extension(connection)?;
    connection
        .execute_batch("ALTER TABLE known_term_publications DROP COLUMN kind;")
        .map_err(super::map)
}

fn remove_relation_extension(connection: &rusqlite::Connection) -> Result<(), LedgerError> {
    if version(connection) >= 9 {
        connection
            .execute_batch("DROP TABLE verified_artifacts;")
            .map_err(super::map)?;
    }
    if version(connection) >= 8 {
        connection
            .execute_batch("DROP TABLE mention_ambiguities; DROP INDEX recordings_capture_order;")
            .map_err(super::map)?;
    }
    Ok(())
}
