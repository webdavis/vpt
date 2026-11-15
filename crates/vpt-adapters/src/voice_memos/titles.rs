mod copying;
mod error;
mod files;
mod lease;
mod query;

use crate::sqlite_open::{self, Mode};
use crate::{ContainedError, RootDir};
use error::DatabaseError;
use lease::CopyLease;
use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;
use vpt_application::ports::TitleLookup;

pub const COPY_DIRECTORY: &str = "title-copy";
const DATABASE: &str = "CloudRecordings.db";
const SIDE_FILES: [&str; 3] = [DATABASE, "CloudRecordings.db-wal", "CloudRecordings.db-shm"];
const TIMEOUT: Duration = Duration::from_secs(5);

pub struct TitleCopy {
    database: Option<Connection>,
    table: Option<String>,
    lease: Option<CopyLease>,
}

impl TitleCopy {
    pub fn refresh(container: &Path, state_dir: &Path) -> Result<Self, ContainedError> {
        match RootDir::open(state_dir) {
            Ok(state) => Self::refresh_at(container, &state),
            Err(error) => Self::outcome(Err(error.into())),
        }
    }

    pub(crate) fn refresh_at(container: &Path, state: &RootDir) -> Result<Self, ContainedError> {
        Self::refresh_with_timeout(container, state, TIMEOUT)
    }

    fn refresh_with_timeout(
        container: &Path,
        state: &RootDir,
        timeout: Duration,
    ) -> Result<Self, ContainedError> {
        Self::outcome((|| {
            let lease = copying::refresh(container, state, timeout)?;
            Self::open_copy(Mode::ReadOnlyOwnIndex, lease)
        })())
    }

    pub fn existing(state_dir: &Path) -> Result<Self, ContainedError> {
        Self::outcome((|| {
            let state = RootDir::open(state_dir)?;
            let root = state.open_subdirectory(COPY_DIRECTORY)?;
            let lease = CopyLease::read(&root, TIMEOUT)?;
            Self::open_copy(Mode::ReadOnly, lease)
        })())
    }

    fn unavailable() -> Self {
        Self {
            database: None,
            table: None,
            lease: None,
        }
    }

    fn outcome(result: Result<Self, DatabaseError>) -> Result<Self, ContainedError> {
        match result {
            Ok(copy) => Ok(copy),
            Err(DatabaseError::Contained(
                error @ (ContainedError::Escape { .. }
                | ContainedError::NotRegular(_)
                | ContainedError::NotADirectory(_)),
            )) => Err(error),
            Err(_) => Ok(Self::unavailable()),
        }
    }

    fn open_copy(mode: Mode, mut lease: CopyLease) -> Result<Self, DatabaseError> {
        lease.revalidate()?;
        files::validate(lease.root())?;
        let database = sqlite_open::open(&lease.root().path().join(DATABASE), mode, TIMEOUT)?;
        let Some(table) = query::table(&database)? else {
            return Ok(Self::unavailable());
        };
        if mode == Mode::ReadOnlyOwnIndex {
            lease.downgrade()?;
        }
        Ok(Self {
            database: Some(database),
            table: Some(table),
            lease: Some(lease),
        })
    }

    pub fn title(&self, file_name: &str) -> TitleLookup {
        let (Some(database), Some(table), Some(lease)) = (&self.database, &self.table, &self.lease)
        else {
            return TitleLookup::Unavailable;
        };
        let result = (|| {
            lease.revalidate()?;
            files::validate(lease.root())?;
            query::title(database, table, file_name).map_err(DatabaseError::from)
        })();
        match result {
            Ok(Some(title)) if !title.trim().is_empty() => TitleLookup::Titled(title),
            Ok(_) | Err(_) => TitleLookup::Unavailable,
        }
    }
}

impl Drop for TitleCopy {
    fn drop(&mut self) {
        drop(self.database.take());
        drop(self.lease.take());
    }
}

#[cfg(test)]
pub(super) mod tests;
