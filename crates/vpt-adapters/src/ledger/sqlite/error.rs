use crate::ContainedError;
use crate::database::DatabaseError;
use rusqlite::ErrorCode;
use vpt_application::ports::LedgerError;

pub(crate) fn map(error: rusqlite::Error) -> LedgerError {
    if let rusqlite::Error::SqliteFailure(error, _) = error {
        match error.code {
            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => return LedgerError::Busy,
            ErrorCode::ConstraintViolation => {
                return LedgerError::Conflict("database constraint violated".into());
            }
            ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase => {
                return LedgerError::Corrupt("database is corrupt or invalid".into());
            }
            _ => {}
        }
    }
    LedgerError::Io("database operation failed".into())
}

pub(super) fn contained(error: ContainedError) -> LedgerError {
    match error {
        ContainedError::Escape { path, .. }
        | ContainedError::NotRegular(path)
        | ContainedError::NotADirectory(path) => LedgerError::PathEscape(path),
        ContainedError::Io { path, kind } => {
            LedgerError::Io(format!("{kind:?} at {}", path.display()))
        }
    }
}

impl From<DatabaseError> for LedgerError {
    fn from(error: DatabaseError) -> Self {
        match error {
            DatabaseError::Contained(error) => contained(error),
            DatabaseError::Sqlite(error) => map(error),
            DatabaseError::Io { operation, kind } => Self::Io(format!("{operation}: {kind:?}")),
        }
    }
}

pub(super) fn finish<T>(
    result: Result<T, LedgerError>,
    closed: Result<(), LedgerError>,
) -> Result<T, LedgerError> {
    match (result, closed) {
        (_, Err(error @ LedgerError::PathEscape(_))) => Err(error),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

#[cfg(test)]
mod tests;
