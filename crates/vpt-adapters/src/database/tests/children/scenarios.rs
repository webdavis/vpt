use crate::database::files::DatabaseFiles;
use crate::database::{AccessPolicy, FileRole};
use crate::{ContainedError, RootDir};
use std::ffi::CString;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(in crate::database::tests) fn ready() {
    println!("\nVPT_READY");
    io::stdout().flush().unwrap();
    let mut command = String::new();
    io::stdin().read_line(&mut command).unwrap();
    assert_eq!(command, "continue\n");
}

fn native(path: &Path) -> rusqlite::Connection {
    // SAFETY: SQLite owns the registration and the name is a static C string.
    assert!(
        !unsafe { rusqlite::ffi::sqlite3_vfs_find(c"unix-posix".as_ptr()) }.is_null(),
        "bundled SQLite must register the POSIX record-lock reference filesystem"
    );
    let connection = rusqlite::Connection::open_with_flags_and_vfs(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        "unix-posix",
    )
    .unwrap();
    connection.busy_timeout(std::time::Duration::ZERO).unwrap();
    connection
}

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

#[test]
fn database_child() {
    let Ok(scenario) = std::env::var("VPT_DATABASE_SCENARIO") else {
        return;
    };
    if let Some(expected) = std::env::var_os("VPT_DATABASE_ROOT_PID") {
        // SAFETY: getppid only observes the parent process identifier.
        let parent = unsafe { libc::getppid() };
        assert_eq!(
            parent.to_string(),
            expected
                .into_string()
                .expect("root test process identifier"),
            "database participant must be a direct child of the root test"
        );
    }
    if std::env::var_os("VPT_PROBE").is_some() {
        eprintln!(
            "VPT_EPOCH_US {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        );
    }
    let directory = std::env::var_os("VPT_DATABASE_ROOT").expect("owned root");
    let opening = crate::ledger::schema_diagnostics::start("root.open.start");
    let root = RootDir::open(&Path::new(&directory).join("state")).unwrap();
    crate::ledger::schema_diagnostics::end(opening, "root.open.end");
    match scenario.as_str() {
        "ledger_contract_history_budget_exit"
        | "ledger_contract_history_budget_stall"
        | "budget_exit" => super::budget::finite_child(&scenario, Path::new(&directory)),
        "identity_exit" | "identity_signal" | "identity_grace" => {
            super::identity::finite_child(&scenario, Path::new(&directory))
        }
        "supervision_ready" => ready(),
        "supervision_twice" => {
            ready();
            ready();
        }
        "supervision_stall" => {
            ready();
            eprintln!("VPT_PHASE pre-sql-stall");
            loop {
                std::thread::park();
            }
        }
        s if s.starts_with("wal_") => super::super::wal::run(s, root),
        s if s.starts_with("ledger_") => crate::ledger::run_scenario(s, root, ready, native),
        s if s.starts_with("lock_") => crate::lock::tests::scenarios::run(s, root, ready),
        s if s.starts_with("recorder_") => crate::voice_memos::run_scenario(s, root),
        s if s.starts_with("title_") => crate::voice_memos::run_title_scenario(s, root, ready),
        s if s.starts_with("failure_") => super::super::failures::run(s, root),
        s if s.starts_with("shm_") => super::super::shared_memory::run(s, root),
        "native_wal_writer"
        | "native_reader"
        | "native_wal_contender"
        | "rooted_snapshot_writer"
        | "rooted_wal_writer"
        | "rooted_committed"
        | "rooted_uncommitted"
        | "rooted_rollback" => {
            super::super::wal::scenarios::participant(&scenario, root, ready, native)
        }
        "close_description" => super::super::locks::close_description(root),
        "owner_failed_open" => super::super::owner::failed_open(root),
        "owner_retry_close" => super::super::owner::retry_close(root),
        "owner_invalid_drop" => super::super::owner::invalid_drop(root, ready),
        "owner_latches" => super::super::owner::error_latches(root),
        "owner_exclusive_create" | "owner_controls" | "owner_readonly" | "owner_path_access" => {
            super::super::owner::controls_case(&scenario, root)
        }
        "owner_transaction_sync"
        | "owner_transaction_directory"
        | "owner_transaction_short"
        | "owner_transaction_write" => super::super::owner::transaction_failure(&scenario, root),
        "owner_short_read"
        | "owner_partial_write"
        | "owner_invalid_offsets"
        | "owner_unknown_open"
        | "owner_root_switch"
        | "owner_panic"
        | "owner_delete_sync"
        | "owner_faults" => super::super::owner::callback_case(&scenario, root),
        "stall" => {
            // SAFETY: this supervised fixture ignores termination to exercise forced cleanup.
            unsafe {
                libc::signal(libc::SIGTERM, libc::SIG_IGN);
            }
            println!("OUT\t\u{1b}{}", "o".repeat(1024));
            io::stdout().flush().unwrap();
            eprintln!(
                "ERR\t\u{1b}{}\nVPT_PHASE stall-start\nVPT_PHASE stall-wait",
                "e".repeat(1024)
            );
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            panic!("stall fixture unexpectedly received input");
        }
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
