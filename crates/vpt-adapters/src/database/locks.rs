use super::DatabaseError;
use rusqlite::ffi::{
    SQLITE_LOCK_EXCLUSIVE as EXCLUSIVE, SQLITE_LOCK_NONE as NONE, SQLITE_LOCK_PENDING as PENDING,
    SQLITE_LOCK_RESERVED as RESERVED, SQLITE_LOCK_SHARED as SHARED,
};
use std::fs::File;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;

const PENDING_BYTE: i64 = 0x4000_0000;
const RESERVED_BYTE: i64 = PENDING_BYTE + 1;
const SHARED_FIRST: i64 = PENDING_BYTE + 2;
const SHARED_SIZE: i64 = 510;

pub(crate) enum LockKind {
    Shared,
    Exclusive,
    Unlock,
}

pub(crate) fn set_record_lock(
    file: &File,
    start: i64,
    length: i64,
    kind: LockKind,
) -> Result<(), DatabaseError> {
    record_lock(file, start, length, kind, libc::F_OFD_SETLK).map(|_| ())
}

fn has_conflict(file: &File, start: i64, length: i64) -> Result<bool, DatabaseError> {
    conflicting_lock(file, start, length).map(|kind| !matches!(kind, LockKind::Unlock))
}

pub(super) fn conflicting_lock(
    file: &File,
    start: i64,
    length: i64,
) -> Result<LockKind, DatabaseError> {
    let lock = record_lock(file, start, length, LockKind::Exclusive, libc::F_OFD_GETLK)?;
    match lock.l_type {
        libc::F_UNLCK => Ok(LockKind::Unlock),
        libc::F_RDLCK => Ok(LockKind::Shared),
        libc::F_WRLCK => Ok(LockKind::Exclusive),
        _ => Err(invalid()),
    }
}

fn record_lock(
    file: &File,
    start: i64,
    length: i64,
    kind: LockKind,
    command: libc::c_int,
) -> Result<libc::flock, DatabaseError> {
    if start < 0 || length < 0 || start.checked_add(length).is_none() {
        return Err(invalid());
    }
    // SAFETY: flock is an integer-only C structure; all-zero fields are valid.
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_whence = libc::SEEK_SET as libc::c_short;
    lock.l_start = start;
    lock.l_len = length;
    lock.l_type = match kind {
        LockKind::Shared => libc::F_RDLCK,
        LockKind::Exclusive => libc::F_WRLCK,
        LockKind::Unlock => libc::F_UNLCK,
    };
    loop {
        // SAFETY: the owned file and initialized flock remain live for this nonblocking call.
        if unsafe { libc::fcntl(file.as_raw_fd(), command, &mut lock) } != -1 {
            return Ok(lock);
        }
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::EACCES | libc::EAGAIN) => {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    None,
                )
                .into());
            }
            _ => {
                return Err(DatabaseError::Io {
                    operation: "record lock",
                    kind: error.kind(),
                });
            }
        }
    }
}

pub(super) struct DatabaseLocks {
    file: File,
    level: i32,
}

impl DatabaseLocks {
    pub(super) fn new(file: File) -> Self {
        Self { file, level: NONE }
    }

    pub(super) fn lock(&mut self, level: i32) -> Result<(), DatabaseError> {
        if !matches!(level, SHARED | RESERVED | EXCLUSIVE)
            || (self.level == NONE && level != SHARED)
        {
            return Err(invalid());
        }
        if self.level >= level {
            return Ok(());
        }
        match level {
            SHARED => {
                set_record_lock(&self.file, PENDING_BYTE, 1, LockKind::Shared)?;
                let shared =
                    set_record_lock(&self.file, SHARED_FIRST, SHARED_SIZE, LockKind::Shared);
                set_record_lock(&self.file, PENDING_BYTE, 1, LockKind::Unlock)?;
                shared?;
            }
            RESERVED => set_record_lock(&self.file, RESERVED_BYTE, 1, LockKind::Exclusive)?,
            EXCLUSIVE => {
                if self.level < PENDING {
                    set_record_lock(&self.file, PENDING_BYTE, 1, LockKind::Exclusive)?;
                    self.level = PENDING;
                }
                set_record_lock(&self.file, SHARED_FIRST, SHARED_SIZE, LockKind::Exclusive)?;
            }
            _ => unreachable!(),
        }
        self.level = level;
        Ok(())
    }

    pub(super) fn unlock(&mut self, level: i32) -> Result<(), DatabaseError> {
        if !matches!(level, NONE | SHARED) {
            return Err(invalid());
        }
        if self.level <= level {
            return Ok(());
        }
        if level == NONE {
            set_record_lock(&self.file, 0, 0, LockKind::Unlock)?;
        } else {
            set_record_lock(&self.file, SHARED_FIRST, SHARED_SIZE, LockKind::Shared)?;
            set_record_lock(&self.file, PENDING_BYTE, 2, LockKind::Unlock)?;
        }
        self.level = level;
        Ok(())
    }

    pub(super) fn reserved(&self) -> Result<bool, DatabaseError> {
        if self.level >= RESERVED {
            Ok(true)
        } else {
            has_conflict(&self.file, RESERVED_BYTE, 1)
        }
    }

    pub(super) fn file(&self) -> &File {
        &self.file
    }

    pub(super) fn level(&self) -> i32 {
        self.level
    }
}

fn invalid() -> DatabaseError {
    DatabaseError::Io {
        operation: "record lock",
        kind: ErrorKind::InvalidInput,
    }
}
