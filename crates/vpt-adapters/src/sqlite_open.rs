use rusqlite::config::DbConfig;
use rusqlite::{Connection, OpenFlags};
use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Writable,
    ReadOnly,
    /// Reads, and may rebuild the shared-memory index beside a private copy.
    ReadOnlyOwnIndex,
}

/// Opens a database through SQLite's own file access, refusing a symbolic link at the path.
/// A writable open creates a missing file; a read-only open creates and writes nothing.
pub(crate) fn open(path: &Path, mode: Mode, busy: Duration) -> rusqlite::Result<Connection> {
    let (query, access) = match mode {
        Mode::Writable => (
            "",
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        ),
        Mode::ReadOnly => ("?mode=ro&readonly_shm=1", OpenFlags::SQLITE_OPEN_READ_ONLY),
        Mode::ReadOnlyOwnIndex => ("?mode=ro", OpenFlags::SQLITE_OPEN_READ_ONLY),
    };
    let connection = Connection::open_with_flags(
        uri(path, query),
        access
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW
            | OpenFlags::SQLITE_OPEN_EXRESCODE
            | OpenFlags::SQLITE_OPEN_URI,
    )?;
    connection.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
    connection.busy_timeout(busy)?;
    connection.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA mmap_size=0;")?;
    Ok(connection)
}

fn uri(path: &Path, query: &str) -> OsString {
    let mut bytes = b"file:".to_vec();
    for &byte in path.as_os_str().as_bytes() {
        match byte {
            b'%' | b'?' | b'#' => bytes.extend(format!("%{byte:02x}").bytes()),
            _ => bytes.push(byte),
        }
    }
    bytes.extend(query.bytes());
    OsString::from_vec(bytes)
}
