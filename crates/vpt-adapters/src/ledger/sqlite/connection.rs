use super::{DATABASE, error, map};
use crate::database::{AccessPolicy, Database};
use crate::{Access, ContainedError, RootDir};
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Duration;
use vpt_application::ports::LedgerError;

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
    policy: AccessPolicy,
    retained: Option<&Database>,
) -> Result<Database, LedgerError> {
    let root = state.try_clone().map_err(error::contained)?;
    let mut owner = match retained {
        Some(retained) => Database::open_pinned(root, Path::new(DATABASE), policy, busy, retained),
        None => Database::open(root, Path::new(DATABASE), policy, busy),
    }?;
    let result = owner.with_connection(|connection| {
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(map)?;
        if policy == AccessPolicy::WritableLedger {
            connection
                .pragma_update(None, "journal_mode", "WAL")
                .map_err(map)?;
        }
        Ok(())
    });
    match result {
        Ok(()) => Ok(owner),
        Err(failure) => error::finish(Err(failure), owner.try_close().map_err(Into::into)),
    }
}

pub(super) fn companions(state: &RootDir) -> Result<(), LedgerError> {
    for leaf in ["vpt.db-wal", "vpt.db-shm"] {
        let file = state
            .open_file(Path::new(leaf), Access::Read)
            .map_err(error::contained)?;
        let metadata = file
            .metadata()
            .map_err(|error| LedgerError::Io(format!("file metadata: {:?}", error.kind())))?;
        if metadata.nlink() != 1 {
            return Err(LedgerError::PathEscape(state.path().join(leaf)));
        }
    }
    Ok(())
}
