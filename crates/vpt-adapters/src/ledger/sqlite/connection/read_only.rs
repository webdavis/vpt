use super::super::{DATABASE, error};
use super::FILES;
use crate::{Access, RootDir};
use std::io::ErrorKind;
use std::os::unix::fs::FileExt;
use std::path::Path;
use vpt_application::ports::LedgerError;

const WAL: usize = 2;
const SHM: usize = 3;

/// Refuses a read-only open that SQLite could only serve by creating a file: a write-ahead log
/// with no shared-memory index beside it, or a WAL-mode database with no write-ahead log.
pub(super) fn companions(state: &RootDir) -> Result<(), LedgerError> {
    let side = |index: usize| {
        state
            .single_file(Path::new(FILES[index]))
            .map_err(error::contained)
    };
    match (side(WAL)?, side(SHM)?) {
        (Some(wal), None) if wal.size > 0 => Err(missing(SHM)),
        (None, _) if wal_mode(state)? => Err(missing(WAL)),
        _ => Ok(()),
    }
}

/// Reads the database header before any connection opens: closing this descriptor while one is
/// open would release the record locks SQLite holds on the file.
fn wal_mode(state: &RootDir) -> Result<bool, LedgerError> {
    let file = state
        .open_file(Path::new(DATABASE), Access::Read)
        .map_err(error::contained)?;
    let mut versions = [0; 2];
    match file.read_exact_at(&mut versions, 18) {
        Ok(()) => Ok(versions == [2, 2]),
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(LedgerError::Io(format!("file read: {:?}", error.kind()))),
    }
}

fn missing(index: usize) -> LedgerError {
    LedgerError::Io(format!("{} missing", FILES[index]))
}
