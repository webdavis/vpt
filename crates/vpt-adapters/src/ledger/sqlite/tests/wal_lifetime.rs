use super::{seed, supervised};
use crate::database::{LockKind, set_record_lock};
use crate::{Access, RootDir, SqliteLedger};
use std::os::unix::fs::FileExt;
use std::path::Path;
use vpt_application::ports::{HistoryWrite, LedgerError, RunHistory};
use vpt_domain::{run::RunKind, time::UtcInstant};

#[test]
fn retained_wal_index_refuses_a_replaced_main_file_without_writes() {
    supervised("ledger_wal_lifetime_replace_main");
}

#[test]
fn retained_wal_index_refuses_a_replaced_wal_file_without_writes() {
    supervised("ledger_wal_lifetime_replace_wal");
}

#[test]
fn retained_wal_index_refuses_a_replaced_shm_file_without_writes() {
    supervised("ledger_wal_lifetime_replace_shm");
}

#[test]
fn retained_wal_index_refuses_a_missing_main_file_without_recreating_it() {
    supervised("ledger_wal_lifetime_missing_main");
}

#[test]
fn retained_wal_index_refuses_a_missing_wal_file_without_recreating_it() {
    supervised("ledger_wal_lifetime_missing_wal");
}

#[test]
fn retained_wal_index_refuses_a_missing_shm_file_without_recreating_it() {
    supervised("ledger_wal_lifetime_missing_shm");
}

#[test]
fn writable_ledger_preserves_initialized_wal_index_between_operations() {
    supervised("ledger_wal_lifetime_reuse");
}

#[test]
fn writable_ledger_releases_wal_lifetime_on_drop() {
    supervised("ledger_wal_lifetime_drop");
}

#[test]
fn observational_ledger_leaves_no_wal_lifetime_lock_between_queries() {
    supervised("ledger_wal_lifetime_observer");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    let ledger = seed(root);
    ledger
        .transaction(|connection| {
            connection
                .execute(
                    "INSERT INTO dirty_publications VALUES ('lifetime', NULL, 'digest', 1)",
                    [],
                )
                .map_err(super::map)?;
            Ok(())
        })
        .unwrap();
    if let Some(role) = scenario
        .strip_prefix("ledger_wal_lifetime_replace_")
        .or_else(|| scenario.strip_prefix("ledger_wal_lifetime_missing_"))
    {
        let name = match role {
            "main" => "vpt.db",
            "wal" => "vpt.db-wal",
            "shm" => "vpt.db-shm",
            _ => unreachable!(),
        };
        let path = root.path().join(name);
        let held = root.path().join("replaced-file");
        std::fs::rename(&path, &held).unwrap();
        if scenario.contains("_replace_") {
            std::fs::copy(&held, &path).unwrap();
        }
        let mut before = crate::database::snapshot(root.path());
        assert_eq!(
            ledger.schema_version(),
            Err(LedgerError::PathEscape(path.clone()))
        );
        assert_eq!(
            ledger.begin_run(RunKind::Ingest, UtcInstant { secs: 10 }),
            HistoryWrite::Unconfirmed(LedgerError::PathEscape(path)),
        );
        assert!(
            crate::database::snapshot(root.path()) == before,
            "refusal before drop changed database files"
        );
        drop(ledger);
        let mut after = crate::database::snapshot(root.path());
        let mapped = if role == "shm" {
            held
        } else {
            root.path().join("vpt.db-shm")
        };
        for entry in before.iter_mut().chain(&mut after) {
            entry.ignore_mapping_release_time(&mapped);
        }
        assert!(
            after == before,
            "dropping the ledger changed database files beyond mapping release time"
        );
        return;
    }
    let shm = root
        .open_file(Path::new("vpt.db-shm"), Access::ReadWrite)
        .unwrap();
    if scenario == "ledger_wal_lifetime_reuse" {
        let mut before = [0; 96];
        shm.read_exact_at(&mut before, 0).unwrap();
        assert_eq!(
            ledger.schema_version().unwrap(),
            super::super::SCHEMA_VERSION
        );
        let mut after = [0; 96];
        shm.read_exact_at(&mut after, 0).unwrap();
        assert_eq!(
            after, before,
            "reading must reuse the initialized WAL index"
        );
    } else if scenario == "ledger_wal_lifetime_drop" {
        set_record_lock(&shm, 120, 8, LockKind::Exclusive).unwrap();
        set_record_lock(&shm, 120, 8, LockKind::Unlock).unwrap();
        assert!(set_record_lock(&shm, 128, 1, LockKind::Exclusive).is_err());
        drop(ledger);
        set_record_lock(&shm, 128, 1, LockKind::Exclusive).unwrap();
    } else {
        drop(ledger);
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        observer.schema_version().unwrap();
        set_record_lock(&shm, 128, 1, LockKind::Exclusive).unwrap();
    }
}
