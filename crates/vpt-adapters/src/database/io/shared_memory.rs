use super::super::{
    invalid,
    shared_memory::{MappedRegion, SharedMemory},
};
use super::{FileRole, run};
use rusqlite::ffi::*;
use std::{ffi::c_void, ptr};

pub(super) unsafe extern "C" fn map(
    file: *mut sqlite3_file,
    region: i32,
    size: i32,
    extend: i32,
    output: *mut *mut c_void,
) -> i32 {
    // SAFETY: SQLite owns writable output storage; run validates file and registration state.
    unsafe {
        if !output.is_null() {
            *output = ptr::null_mut();
        }
        run(file, SQLITE_IOERR_SHMMAP, |file, state| {
            if output.is_null() || file.role != FileRole::Database {
                return Err(invalid("shared memory map arguments"));
            }
            if file.shared_memory.is_none() {
                file.shared_memory = Some(SharedMemory::open(&state.files)?);
            }
            let shared = file
                .shared_memory
                .as_mut()
                .ok_or_else(|| invalid("shared memory state"))?;
            match shared.map(region, size, extend != 0)? {
                MappedRegion::Missing => Ok(SQLITE_OK),
                MappedRegion::ReadOnlyUninitialized => Ok(SQLITE_READONLY_CANTINIT),
                MappedRegion::ReadOnly(address) => {
                    *output = address.as_ptr();
                    Ok(SQLITE_READONLY)
                }
                MappedRegion::Writable(address) => {
                    *output = address.as_ptr();
                    Ok(SQLITE_OK)
                }
            }
        })
    }
}

pub(super) unsafe extern "C" fn lock(
    file: *mut sqlite3_file,
    offset: i32,
    count: i32,
    flags: i32,
) -> i32 {
    // SAFETY: run checks the SQLite-owned header and callback state.
    unsafe {
        run(file, SQLITE_IOERR_SHMLOCK, |file, _| {
            file.shared_memory
                .as_mut()
                .ok_or_else(|| invalid("shared memory lock state"))?
                .lock(offset, count, flags)?;
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn barrier(file: *mut sqlite3_file) {
    // SAFETY: run keeps the ordering callback contained within the C boundary.
    unsafe {
        run(file, SQLITE_IOERR_SHMLOCK, |_, _| {
            std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
            Ok(SQLITE_OK)
        });
    }
}

pub(super) unsafe extern "C" fn unmap(file: *mut sqlite3_file, delete: i32) -> i32 {
    // SAFETY: run checks the SQLite-owned header and callback state.
    unsafe {
        run(file, SQLITE_IOERR_SHMMAP, |file, _| {
            if let Some(shared) = file.shared_memory.take() {
                shared.unmap(delete != 0 && !file.persistent_wal)?;
            }
            Ok(SQLITE_OK)
        })
    }
}
