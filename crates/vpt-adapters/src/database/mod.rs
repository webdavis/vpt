mod files;
mod locks;

use crate::ContainedError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "D3 selects connection file policies")
)]
pub(crate) enum AccessPolicy {
    WritableLedger,
    ObservationalLedger,
    RefreshedTitleCopy,
    ExistingTitleCopy,
}

impl AccessPolicy {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D3 enforces callback write policies")
    )]
    fn writable(self, role: FileRole) -> bool {
        match self {
            Self::WritableLedger => true,
            Self::RefreshedTitleCopy => role == FileRole::SharedMemory,
            Self::ObservationalLedger | Self::ExistingTitleCopy => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "D3 validates callback file roles")
)]
enum FileRole {
    Database,
    Journal,
    Wal,
    SharedMemory,
}

#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "D3 retains typed connection failures")
)]
pub(crate) enum DatabaseError {
    Contained(ContainedError),
    Sqlite(rusqlite::Error),
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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "D3 classifies connection containment failures")
    )]
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
