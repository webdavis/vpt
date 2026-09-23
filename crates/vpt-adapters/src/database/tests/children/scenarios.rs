use crate::database::files::DatabaseFiles;
use crate::database::{AccessPolicy, FileRole};
use crate::{ContainedError, RootDir};
use std::ffi::CString;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn ready() {
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
    let directory = std::env::var_os("VPT_DATABASE_ROOT").expect("owned root");
    let root = RootDir::open(&Path::new(&directory).join("state")).unwrap();
    match scenario.as_str() {
        "close_description" => super::super::locks::close_description(root),
        "stall" => {
            // SAFETY: this supervised fixture ignores termination to exercise forced cleanup.
            unsafe {
                libc::signal(libc::SIGTERM, libc::SIG_IGN);
            }
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
