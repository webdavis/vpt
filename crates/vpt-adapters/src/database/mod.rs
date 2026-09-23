mod faults;
mod files;
mod io;
mod locks;
mod owner;
mod registration;
mod state;

#[cfg_attr(
    not(test),
    expect(unused_imports, reason = "D5 imports the contained ledger owner")
)]
pub(crate) use owner::Database;

use crate::ContainedError;

fn io_error(operation: &'static str, error: std::io::Error) -> DatabaseError {
    DatabaseError::Io {
        operation,
        kind: error.kind(),
    }
}

fn sqlite_error(code: i32) -> DatabaseError {
    rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None).into()
}

fn invalid(operation: &'static str) -> DatabaseError {
    DatabaseError::Io {
        operation,
        kind: std::io::ErrorKind::InvalidInput,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessPolicy {
    WritableLedger,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D5 constructs observational ledger connections")
    )]
    ObservationalLedger,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D6 constructs refreshed title connections")
    )]
    RefreshedTitleCopy,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D6 constructs existing title connections")
    )]
    ExistingTitleCopy,
}

impl AccessPolicy {
    fn writable(self, role: FileRole) -> bool {
        match self {
            Self::WritableLedger => true,
            Self::RefreshedTitleCopy => role == FileRole::SharedMemory,
            Self::ObservationalLedger | Self::ExistingTitleCopy => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileRole {
    Database,
    Journal,
    Wal,
    SharedMemory,
}

#[derive(Debug)]
pub(crate) enum DatabaseError {
    Contained(ContainedError),
    Sqlite(rusqlite::Error),
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D5 maps typed storage failures")
    )]
    Io {
        operation: &'static str,
        kind: std::io::ErrorKind,
    },
}

impl From<ContainedError> for DatabaseError {
    fn from(error: ContainedError) -> Self {
        Self::Contained(error)
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl DatabaseError {
    fn is_containment(&self) -> bool {
        matches!(
            self,
            Self::Contained(
                ContainedError::Escape { .. }
                    | ContainedError::NotRegular(_)
                    | ContainedError::NotADirectory(_)
            )
        )
    }
}

#[cfg(test)]
mod tests;
