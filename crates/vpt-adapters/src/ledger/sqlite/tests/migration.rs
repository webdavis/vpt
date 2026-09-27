use super::super::{error, migrations};
use super::{database, supervised, tables, version};
use crate::RootDir;
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

pub(super) fn run(scenario: &str, root: &RootDir) {
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
                // SAFETY: the borrowed connection retains this live SQLite handle.
                assert_eq!(
                    unsafe {
                        rusqlite::ffi::sqlite3_txn_state(connection.handle(), std::ptr::null())
                    },
                    2
                );
                assert_eq!(version(connection), 1);
                assert_eq!(tables(connection).len(), 11);
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
                assert_eq!(migrations::current(connection).unwrap(), 1);
                assert_eq!(tables(connection).len(), 11);
                migrations::migrate(connection, || Ok(())).unwrap();
                assert_eq!(tables(connection).len(), 11);
            }
        }
        Ok(())
    })
    .unwrap();
}
