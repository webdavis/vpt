use crate::ContainedError;

#[derive(Debug)]
pub(crate) enum DatabaseError {
    Contained(ContainedError),
    Busy,
    Unavailable,
}

impl From<ContainedError> for DatabaseError {
    fn from(error: ContainedError) -> Self {
        Self::Contained(error)
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::DatabaseBusy) => Self::Busy,
            _ => Self::Unavailable,
        }
    }
}
