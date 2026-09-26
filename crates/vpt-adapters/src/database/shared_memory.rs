mod mapping;

use super::locks::{LockKind, conflicting_lock, set_record_lock};
use super::{DatabaseError, FileRole, files::DatabaseFiles, io_error, sqlite_error};
use mapping::Region;
use rusqlite::ffi::SQLITE_BUSY;
use std::{ffi::c_void, fs::File, ptr::NonNull, rc::Rc};

const SHM_LIFETIME_BYTE: i64 = 128;
const SHM_REGION_BYTES: usize = 32 * 1024;

enum Initialization {
    Acquired,
    ReadOnlyUninitialized,
}

#[derive(Debug)]
pub(super) enum MappedRegion {
    Missing,
    ReadOnlyUninitialized,
    ReadOnly(NonNull<c_void>),
    Writable(NonNull<c_void>),
}

pub(super) struct SharedMemory {
    files: Rc<DatabaseFiles>,
    file: File,
    writable: bool,
    initialization: Initialization,
    mappings: Vec<Region>,
    held_slots: [i32; 8],
}

impl SharedMemory {
    pub(super) fn open(files: &Rc<DatabaseFiles>) -> Result<Self, DatabaseError> {
        let file = files.open(FileRole::SharedMemory, true)?;
        let writable = files.policy().writable(FileRole::SharedMemory);
        let initialization = initialize(&file, files, writable)?;
        Ok(Self {
            files: Rc::clone(files),
            file,
            writable,
            initialization,
            mappings: Vec::new(),
            held_slots: [0; 8],
        })
    }

    pub(super) fn map(
        &mut self,
        region: i32,
        size: i32,
        extend: bool,
    ) -> Result<MappedRegion, DatabaseError> {
        if region < 0 || usize::try_from(size).ok() != Some(SHM_REGION_BYTES) {
            return Err(super::invalid("shared memory region"));
        }
        let end = u64::try_from(region)
            .map_err(|_| super::invalid("shared memory region"))?
            .checked_add(1)
            .and_then(|regions| regions.checked_mul(size as u64))
            .ok_or_else(|| super::invalid("shared memory region"))?;
        if matches!(self.initialization, Initialization::ReadOnlyUninitialized) {
            self.initialization = initialize(&self.file, &self.files, self.writable)?;
            if matches!(self.initialization, Initialization::ReadOnlyUninitialized) {
                return Ok(MappedRegion::ReadOnlyUninitialized);
            }
        }
        if let Some(mapped) = self.mappings.iter().find(|mapped| mapped.index == region) {
            return Ok(self.mapped(mapped.address()));
        }
        let length = self
            .file
            .metadata()
            .map_err(|e| io_error("shared memory size", e))?
            .len();
        if length < end {
            if !self.writable {
                return Err(sqlite_error(rusqlite::ffi::SQLITE_READONLY));
            }
            if !extend {
                return Ok(MappedRegion::Missing);
            }
            use std::os::unix::fs::FileExt;
            for page in length / 4096..end / 4096 {
                self.file
                    .write_all_at(&[0], page * 4096 + 4095)
                    .map_err(|e| io_error("shared memory provision", e))?;
            }
        }
        self.files.faults.check("map")?;
        let mapped = Region::map(&self.file, region, self.writable)?;
        let address = mapped.address();
        self.mappings.push(mapped);
        Ok(self.mapped(address))
    }

    fn mapped(&self, address: NonNull<c_void>) -> MappedRegion {
        if self.writable {
            MappedRegion::Writable(address)
        } else {
            MappedRegion::ReadOnly(address)
        }
    }

    pub(super) fn lock(
        &mut self,
        offset: i32,
        count: i32,
        flags: i32,
    ) -> Result<(), DatabaseError> {
        use rusqlite::ffi::{
            SQLITE_SHM_EXCLUSIVE, SQLITE_SHM_LOCK, SQLITE_SHM_SHARED, SQLITE_SHM_UNLOCK,
        };
        if ![
            SQLITE_SHM_LOCK | SQLITE_SHM_SHARED,
            SQLITE_SHM_LOCK | SQLITE_SHM_EXCLUSIVE,
            SQLITE_SHM_UNLOCK | SQLITE_SHM_SHARED,
            SQLITE_SHM_UNLOCK | SQLITE_SHM_EXCLUSIVE,
        ]
        .contains(&flags)
            || offset < 0
            || count < 1
            || offset.checked_add(count).is_none_or(|end| end > 8)
            || (flags & SQLITE_SHM_SHARED != 0 && count != 1)
        {
            return Err(super::invalid("shared memory lock"));
        }
        let kind = if flags & SQLITE_SHM_UNLOCK != 0 {
            LockKind::Unlock
        } else if flags & SQLITE_SHM_EXCLUSIVE != 0 {
            LockKind::Exclusive
        } else {
            LockKind::Shared
        };
        let mode = flags & (SQLITE_SHM_SHARED | SQLITE_SHM_EXCLUSIVE);
        let slots = &mut self.held_slots[offset as usize..(offset + count) as usize];
        if slots.iter().any(|held| *held != 0 && *held != mode) {
            return Err(super::invalid("shared memory lock transition"));
        }
        set_record_lock(&self.file, 120 + i64::from(offset), i64::from(count), kind)?;
        slots.fill(if flags & SQLITE_SHM_UNLOCK != 0 {
            0
        } else {
            mode
        });
        Ok(())
    }

    pub(super) fn unmap(mut self, delete: bool) -> Result<(), DatabaseError> {
        while let Some(region) = self.mappings.pop() {
            region.unmap()?;
        }
        set_record_lock(&self.file, 120, 8, LockKind::Unlock)?;
        self.held_slots.fill(0);
        if delete && self.writable {
            match set_record_lock(&self.file, SHM_LIFETIME_BYTE, 1, LockKind::Exclusive) {
                Ok(()) => self.files.remove(FileRole::SharedMemory, true)?,
                Err(DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(error, _)))
                    if error.code == rusqlite::ErrorCode::DatabaseBusy => {}
                Err(error) => return Err(error),
            }
        }
        set_record_lock(&self.file, SHM_LIFETIME_BYTE, 1, LockKind::Unlock)
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        while let Some(mapped) = self.mappings.pop() {
            drop(mapped);
        }
    }
}

fn initialize(
    file: &File,
    files: &DatabaseFiles,
    writable: bool,
) -> Result<Initialization, DatabaseError> {
    match conflicting_lock(file, SHM_LIFETIME_BYTE, 1)? {
        LockKind::Exclusive => return Err(sqlite_error(SQLITE_BUSY)),
        LockKind::Unlock if !writable => return Ok(Initialization::ReadOnlyUninitialized),
        LockKind::Unlock => {
            match set_record_lock(file, SHM_LIFETIME_BYTE, 1, LockKind::Exclusive) {
                Ok(()) => {
                    let result = files.faults.check("truncate").and_then(|()| {
                        file.set_len(3)
                            .map_err(|e| io_error("shared memory initialize", e))
                    });
                    if let Err(error) = result {
                        set_record_lock(file, SHM_LIFETIME_BYTE, 1, LockKind::Unlock)?;
                        return Err(error);
                    }
                }
                Err(error) => {
                    if !matches!(
                        conflicting_lock(file, SHM_LIFETIME_BYTE, 1)?,
                        LockKind::Shared
                    ) {
                        return Err(error);
                    }
                }
            }
        }
        LockKind::Shared => {}
    }
    set_record_lock(file, SHM_LIFETIME_BYTE, 1, LockKind::Shared)?;
    Ok(Initialization::Acquired)
}
