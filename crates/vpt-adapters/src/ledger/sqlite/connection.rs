use super::{DATABASE, error, map};
use crate::sqlite_open::{self, Mode};
use crate::{ContainedError, RootDir};
use rusqlite::Connection;
use std::fs::Permissions;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;
use vpt_application::ports::LedgerError;

mod read_only;
use read_only::companions;

const FILES: [&str; 4] = [DATABASE, "vpt.db-journal", "vpt.db-wal", "vpt.db-shm"];

/// The device and inode of each database file present, in the order of `FILES`.
pub(super) type Identity = [Option<(u64, u64)>; 4];

pub(super) fn present(state: &RootDir) -> Result<bool, ContainedError> {
    state.revalidate()?;
    let present = match state.regular(Path::new(DATABASE)) {
        Ok(_) => true,
        Err(ContainedError::Io {
            kind: ErrorKind::NotFound,
            ..
        }) => false,
        Err(error) => return Err(error),
    };
    state.revalidate()?;
    Ok(present)
}

pub(super) fn open(
    state: &RootDir,
    busy: Duration,
    mode: Mode,
    retained: Option<&Identity>,
) -> Result<Connection, LedgerError> {
    let found = verify(state, retained)?;
    match mode {
        Mode::Writable if found[0].is_none() => create(state)?,
        Mode::ReadOnly | Mode::ReadOnlyOwnIndex => companions(state)?,
        Mode::Writable => {}
    }
    let connection = open_path(state, busy, mode)?;
    let configured = connection
        .pragma_update(None, "foreign_keys", "ON")
        .and_then(|()| match mode {
            Mode::Writable => connection.pragma_update(None, "journal_mode", "WAL"),
            Mode::ReadOnly | Mode::ReadOnlyOwnIndex => Ok(()),
        })
        .map_err(map);
    match configured {
        Ok(()) => Ok(connection),
        Err(failure) => error::finish(Err(failure), close(connection)),
    }
}

pub(super) fn open_path(
    state: &RootDir,
    busy: Duration,
    mode: Mode,
) -> Result<Connection, LedgerError> {
    sqlite_open::open(&state.path().join(DATABASE), mode, busy).map_err(map)
}

pub(super) fn close(connection: Connection) -> Result<(), LedgerError> {
    connection.close().map_err(|(_, error)| map(error))
}

/// Refuses a database file that is not a regular file with one link, and, given the identity
/// recorded when the ledger opened, one that another file has replaced since.
pub(super) fn verify(
    state: &RootDir,
    retained: Option<&Identity>,
) -> Result<Identity, LedgerError> {
    state.revalidate().map_err(error::contained)?;
    let mut found = [None; 4];
    for (slot, leaf) in found.iter_mut().zip(FILES) {
        *slot = state
            .single_file(Path::new(leaf))
            .map_err(error::contained)?
            .map(|file| file.identity);
    }
    if let Some(retained) = retained {
        for ((leaf, was), now) in FILES.iter().zip(retained).zip(found) {
            if was.is_some() && *was != now {
                return Err(LedgerError::PathEscape(state.path().join(leaf)));
            }
        }
    }
    Ok(found)
}

/// Gives every database file present mode 0600. It changes modes by name, so no descriptor close
/// releases the record locks another connection in this process holds.
pub(super) fn private(state: &RootDir) -> Result<(), LedgerError> {
    for leaf in FILES {
        let path = Path::new(leaf);
        if let Some(file) = state.single_file(path).map_err(error::contained)?
            && file.mode != 0o600
        {
            std::fs::set_permissions(state.path().join(path), Permissions::from_mode(0o600))
                .map_err(|error| LedgerError::Io(format!("file mode: {:?}", error.kind())))?;
        }
    }
    Ok(())
}

fn create(state: &RootDir) -> Result<(), LedgerError> {
    match state.create_read_write_file(Path::new(DATABASE), 0o600) {
        Ok(_)
        | Err(ContainedError::Io {
            kind: ErrorKind::AlreadyExists,
            ..
        }) => Ok(()),
        Err(error) => Err(error::contained(error)),
    }
}
