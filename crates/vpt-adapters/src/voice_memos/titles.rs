mod copying;
mod files;
mod lease;
mod query;

use crate::database::{AccessPolicy, Database, DatabaseError};
use crate::{ContainedError, RootDir};
use lease::CopyLease;
use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;
use vpt_application::ports::TitleLookup;

pub const COPY_DIRECTORY: &str = "title-copy";
const DATABASE: &str = "CloudRecordings.db";
const SIDE_FILES: [&str; 3] = [DATABASE, "CloudRecordings.db-wal", "CloudRecordings.db-shm"];
const TIMEOUT: Duration = Duration::from_secs(5);

pub struct TitleCopy {
    database: Option<RefCell<Database>>,
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
            Self::open_copy(
                lease.root().try_clone()?,
                AccessPolicy::RefreshedTitleCopy,
                lease,
            )
        })())
    }

    pub fn existing(state_dir: &Path) -> Result<Self, ContainedError> {
        Self::outcome((|| {
            let state = RootDir::open(state_dir)?;
            let root = state.open_subdirectory(COPY_DIRECTORY)?;
            let lease = CopyLease::read(&root, TIMEOUT)?;
            Self::open_copy(root, AccessPolicy::ExistingTitleCopy, lease)
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

    fn open_copy(
        root: RootDir,
        policy: AccessPolicy,
        lease: CopyLease,
    ) -> Result<Self, DatabaseError> {
        lease.revalidate()?;
        files::validate(&root)?;
        let database = Database::open(root, Path::new(DATABASE), policy, TIMEOUT)?;
        let mut copy = Self {
            database: Some(RefCell::new(database)),
            table: None,
            lease: Some(lease),
        };
        copy.table = copy
            .database
            .as_mut()
            .unwrap()
            .get_mut()
            .with_connection::<_, DatabaseError>(|connection| {
                query::table(connection).map_err(Into::into)
            })?;
        if copy.table.is_none() {
            return Ok(Self::unavailable());
        }
        if policy == AccessPolicy::RefreshedTitleCopy {
            copy.lease.as_mut().unwrap().downgrade()?;
        }
        Ok(copy)
    }

    pub fn title(&self, file_name: &str) -> TitleLookup {
        let (Some(database), Some(table), Some(lease)) = (&self.database, &self.table, &self.lease)
        else {
            return TitleLookup::Unavailable;
        };
        let result = (|| {
            lease.revalidate()?;
            files::validate(lease.root())?;
            database
                .borrow_mut()
                .with_connection::<_, DatabaseError>(|connection| {
                    query::title(connection, table, file_name).map_err(Into::into)
                })
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
