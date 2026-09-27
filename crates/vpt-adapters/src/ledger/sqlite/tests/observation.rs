use super::super::{OpenError, SqliteLedger};
use super::{database, seed, supervised};
use crate::{RootDir, database::snapshot};
use std::os::unix::fs::PermissionsExt;
use vpt_application::ports::LedgerError;

#[test]
fn absent_state_is_observed_without_creation() {
    supervised("ledger_observe_absent_state");
}

#[test]
fn absent_database_is_observed_without_creation() {
    supervised("ledger_observe_absent_database");
}

#[test]
fn empty_database_is_observed_without_creation_or_repair() {
    supervised("ledger_observe_empty");
}

#[test]
fn readable_version_zero_needs_no_companions_or_migration() {
    supervised("ledger_observe_zero");
}

#[test]
fn unreadable_zero_header_with_a_live_log_is_not_inferred_empty() {
    supervised("ledger_observe_unreadable_zero");
}

#[test]
fn observation_after_last_writer_close_changes_nothing() {
    supervised("ledger_observe_closed");
}

#[test]
fn populated_ledger_with_missing_log_is_not_repaired() {
    supervised("ledger_observe_missing_wal");
}

#[test]
fn populated_ledger_with_missing_shared_memory_is_not_repaired() {
    supervised("ledger_observe_missing_shm");
}

#[test]
fn observational_queries_refuse_writes_and_preserve_loose_modes() {
    supervised("ledger_observe_readonly");
}

#[test]
fn empty_hardlinked_database_is_refused_without_changes() {
    supervised("ledger_observe_hardlink");
}

#[test]
fn invalid_database_bytes_are_reported_without_inventing_an_empty_ledger() {
    supervised("ledger_observe_corrupt");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    match scenario {
        "ledger_observe_corrupt" => {
            std::fs::write(
                root.path().join("vpt.db"),
                b"private invalid database bytes",
            )
            .unwrap();
            let before = snapshot(root.path());
            let result = SqliteLedger::open_read_only(root.path());
            assert!(
                matches!(result, Err(OpenError::Ledger(LedgerError::Corrupt(_)))),
                "{result:?}"
            );
            assert_eq!(snapshot(root.path()), before);
        }
        "ledger_observe_absent_state" => {
            let before = snapshot(root.path());
            assert!(
                SqliteLedger::open_read_only(&root.path().join("absent"))
                    .unwrap()
                    .is_none()
            );
            assert_eq!(snapshot(root.path()), before);
        }
        "ledger_observe_absent_database" | "ledger_observe_empty" | "ledger_observe_zero" => {
            if scenario == "ledger_observe_empty" {
                std::fs::write(root.path().join("vpt.db"), []).unwrap();
            } else if scenario == "ledger_observe_zero" {
                database(root, |c| {
                    c.execute_batch("CREATE TABLE unmigrated(value)").unwrap();
                    assert_eq!(super::version(c), 0);
                    Ok(())
                })
                .unwrap();
            }
            let before = snapshot(root.path());
            assert!(SqliteLedger::open_read_only(root.path()).unwrap().is_none());
            assert_eq!(snapshot(root.path()), before);
        }
        "ledger_observe_unreadable_zero" => {
            database(root, |c| {
                c.set_db_config(
                    rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
                    true,
                )
                .unwrap();
                c.execute_batch(
                    "PRAGMA journal_mode=WAL; CREATE TABLE pending(value); PRAGMA user_version=1;",
                )
                .unwrap();
                Ok(())
            })
            .unwrap();
            let bytes = std::fs::read(root.path().join("vpt.db")).unwrap();
            assert_eq!(&bytes[60..64], &[0, 0, 0, 0], "version is only in the log");
            std::fs::rename(
                root.path().join("vpt.db-shm"),
                root.path().with_file_name("held-shm"),
            )
            .unwrap();
            let before = snapshot(root.path());
            assert!(SqliteLedger::open_read_only(root.path()).is_err());
            assert_eq!(snapshot(root.path()), before);
        }
        "ledger_observe_hardlink" => {
            let target = root.path().with_file_name("hardlink-target");
            std::fs::write(&target, []).unwrap();
            std::fs::hard_link(&target, root.path().join("vpt.db")).unwrap();
            let before = snapshot(root.path());
            let target_before = snapshot(&target);
            assert!(SqliteLedger::open_read_only(root.path()).is_err());
            assert_eq!(snapshot(root.path()), before);
            assert_eq!(snapshot(&target), target_before);
        }
        _ => populated(scenario, root),
    }
}

fn populated(scenario: &str, root: &RootDir) {
    drop(seed(root));
    if let Some(leaf) = match scenario {
        "ledger_observe_missing_wal" => Some("vpt.db-wal"),
        "ledger_observe_missing_shm" => Some("vpt.db-shm"),
        _ => None,
    } {
        std::fs::rename(
            root.path().join(leaf),
            root.path().with_file_name("held-companion"),
        )
        .unwrap();
        let before = snapshot(root.path());
        assert!(SqliteLedger::open_read_only(root.path()).is_err());
        assert_eq!(snapshot(root.path()), before);
        return;
    }
    if scenario == "ledger_observe_readonly" {
        for leaf in ["vpt.db", "vpt.db-wal", "vpt.db-shm"] {
            std::fs::set_permissions(
                root.path().join(leaf),
                std::fs::Permissions::from_mode(0o644),
            )
            .unwrap();
        }
    }
    let before = snapshot(root.path());
    let observed = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert!(
        snapshot(root.path()) == before,
        "observational open changed the state snapshot"
    );
    assert_eq!(observed.schema_version().unwrap(), 1);
    observed
        .read(|c| {
            assert_eq!(super::version(c), 1);
            assert!(
                snapshot(root.path()) == before,
                "observational query changed the state snapshot"
            );
            assert!(c.execute("DELETE FROM dirty_publications", []).is_err());
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        observed.transaction::<()>(|_| panic!("read-only operation must not run")),
        Err(LedgerError::Io(_))
    ));
    drop(observed);
    assert!(
        snapshot(root.path()) == before,
        "observational close changed the state snapshot"
    );
}
