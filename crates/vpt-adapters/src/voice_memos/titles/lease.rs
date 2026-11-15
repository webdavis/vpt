use super::{DATABASE, files};
use crate::database::DatabaseError;
use crate::record_lock::{LockKind, set_record_lock};
use crate::{Access, RootDir};
use std::fs::File;
use std::io::ErrorKind;
use std::time::{Duration, Instant};

pub(super) struct CopyLease {
    file: File,
    root: RootDir,
}

impl CopyLease {
    pub(super) fn read(copy: &RootDir, timeout: Duration) -> Result<Self, DatabaseError> {
        let file = files::acquire(copy, DATABASE, Access::Read, false)?;
        Self::lock(copy, file, false, timeout)
    }

    pub(super) fn refresh(copy: &RootDir, timeout: Duration) -> Result<Self, DatabaseError> {
        let file = files::acquire(copy, DATABASE, Access::ReadWrite, true)?;
        Self::lock(copy, file, true, timeout)
    }

    fn lock(
        copy: &RootDir,
        file: File,
        exclusive: bool,
        timeout: Duration,
    ) -> Result<Self, DatabaseError> {
        let lease = Self {
            file,
            root: copy.try_clone()?,
        };
        let started = Instant::now();
        loop {
            let kind = if exclusive {
                LockKind::Exclusive
            } else {
                LockKind::Shared
            };
            match set_record_lock(&lease.file, 0, 1, kind) {
                Ok(()) => {
                    lease.revalidate()?;
                    return Ok(lease);
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    if started.elapsed() >= timeout {
                        return Err(busy());
                    }
                    std::thread::sleep(
                        Duration::from_millis(1).min(timeout.saturating_sub(started.elapsed())),
                    );
                }
                Err(error) => return Err(locking(error)),
            }
        }
    }

    pub(super) fn downgrade(&mut self) -> Result<(), DatabaseError> {
        self.revalidate()?;
        set_record_lock(&self.file, 0, 1, LockKind::Shared).map_err(|error| match error.kind() {
            ErrorKind::WouldBlock => busy(),
            _ => locking(error),
        })
    }

    pub(super) fn file(&self) -> &File {
        &self.file
    }

    pub(super) fn revalidate(&self) -> Result<(), DatabaseError> {
        files::verify(&self.root, DATABASE, &self.file).map_err(Into::into)
    }

    pub(super) fn root(&self) -> &RootDir {
        &self.root
    }
}

fn busy() -> DatabaseError {
    rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY), None)
        .into()
}

fn locking(error: std::io::Error) -> DatabaseError {
    DatabaseError::Io {
        operation: "record lock",
        kind: error.kind(),
    }
}
