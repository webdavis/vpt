use super::super::files::DatabaseFiles;
use super::super::{AccessPolicy, FileRole};
use crate::test_support::children::scenarios::{native, ready};
use crate::{ContainedError, RootDir};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn fifo(root: RootDir, role: FileRole, name: &str) {
    let path = CString::new(root.path().join(name).as_os_str().as_bytes()).unwrap();
    // SAFETY: the NUL-terminated path names a child-owned synthetic fixture.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::ExistingTitleCopy).unwrap();
    assert!(matches!(
        files.open(role, false),
        Err(crate::database::DatabaseError::Contained(
            ContainedError::NotRegular(_)
        ))
    ));
}

pub(crate) fn run_scenario(scenario: &str, root: RootDir) {
    match scenario {
        s if s.starts_with("wal_") => super::wal::run(s, root),
        s if s.starts_with("failure_") => super::failures::run(s, root),
        s if s.starts_with("shm_") => super::shared_memory::run(s, root),
        "native_wal_writer"
        | "native_reader"
        | "native_wal_contender"
        | "rooted_snapshot_writer"
        | "rooted_wal_writer"
        | "rooted_committed"
        | "rooted_uncommitted"
        | "rooted_rollback" => super::wal::scenarios::participant(scenario, root, ready, native),
        "close_description" => super::locks::close_description(root),
        "owner_failed_open" => super::owner::failed_open(root),
        "owner_retry_close" => super::owner::retry_close(root),
        "owner_invalid_drop" => super::owner::invalid_drop(root, ready),
        "owner_latches" => super::owner::error_latches(root),
        "owner_exclusive_create" | "owner_controls" | "owner_readonly" | "owner_path_access" => {
            super::owner::controls_case(scenario, root)
        }
        "owner_transaction_sync"
        | "owner_transaction_directory"
        | "owner_transaction_short"
        | "owner_transaction_write" => super::owner::transaction_failure(scenario, root),
        "owner_short_read"
        | "owner_partial_write"
        | "owner_invalid_offsets"
        | "owner_unknown_open"
        | "owner_root_switch"
        | "owner_panic"
        | "owner_delete_sync"
        | "owner_faults" => super::owner::callback_case(scenario, root),
        "fifo_database" => fifo(root, FileRole::Database, "vpt.db"),
        "fifo_journal" => fifo(root, FileRole::Journal, "vpt.db-journal"),
        "fifo_wal" => fifo(root, FileRole::Wal, "vpt.db-wal"),
        "fifo_shared_memory" => fifo(root, FileRole::SharedMemory, "vpt.db-shm"),
        "native_writer" | "native_contender" => {
            let path = root.path().join("vpt.db");
            let fixture = native(&path);
            fixture.execute_batch("PRAGMA journal_mode=DELETE; CREATE TABLE fixture(value); INSERT INTO fixture VALUES(1);").unwrap();
            fixture.close().unwrap();
            let connection = native(&path);
            if scenario == "native_writer" {
                connection.execute_batch("BEGIN EXCLUSIVE").unwrap();
                ready();
                connection.execute_batch("ROLLBACK").unwrap();
            } else {
                ready();
                let result = connection.execute_batch("BEGIN IMMEDIATE");
                assert!(
                    matches!(result, Err(rusqlite::Error::SqliteFailure(code, _)) if code.code == rusqlite::ErrorCode::DatabaseBusy),
                    "{result:?}"
                );
            }
            connection.close().unwrap();
        }
        _ => panic!("unknown owned child scenario"),
    }
}
