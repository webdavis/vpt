mod controls;
mod transfer;

use super::locks::DatabaseLocks;
use super::state::State;
use super::{DatabaseError, FileRole};
use rusqlite::ffi::*;
use std::ptr;

#[repr(C)]
pub(super) struct FileHandle {
    pub(super) base: sqlite3_file,
    pub(super) state: *mut FileState,
}

pub(super) struct FileState {
    pub(super) registration: *const State,
    pub(super) locks: DatabaseLocks,
    pub(super) role: FileRole,
    pub(super) writable: bool,
    pub(super) persistent_wal: bool,
}

pub(super) static METHODS: sqlite3_io_methods = sqlite3_io_methods {
    iVersion: 2,
    xClose: Some(close),
    xRead: Some(transfer::read),
    xWrite: Some(transfer::write),
    xTruncate: Some(transfer::truncate),
    xSync: Some(transfer::sync),
    xFileSize: Some(transfer::size),
    xLock: Some(controls::lock),
    xUnlock: Some(controls::unlock),
    xCheckReservedLock: Some(controls::reserved),
    xFileControl: Some(controls::control),
    xSectorSize: Some(controls::sector),
    xDeviceCharacteristics: Some(controls::characteristics),
    xShmMap: None,
    xShmLock: None,
    xShmBarrier: None,
    xShmUnmap: None,
    xFetch: None,
    xUnfetch: None,
};

unsafe fn run(
    file: *mut sqlite3_file,
    failure: i32,
    operation: impl FnOnce(&mut FileState, &State) -> Result<i32, DatabaseError>,
) -> i32 {
    // SAFETY: xOpen initialized this header; NOMUTEX and Database's thread confinement serialize calls.
    let Some(file) = (unsafe { file.cast::<FileHandle>().as_mut() }) else {
        return failure;
    };
    let Some(owned) = (unsafe { file.state.as_mut() }) else {
        return failure;
    };
    // SAFETY: Database retains this pinned registration until every file has closed.
    let Some(registration) = (unsafe { owned.registration.as_ref() }) else {
        return failure;
    };
    registration.run(failure, || operation(owned, registration))
}

unsafe extern "C" fn close(file: *mut sqlite3_file) -> i32 {
    // SAFETY: the header is SQLite-owned storage initialized by xOpen.
    let Some(file) = (unsafe { file.cast::<FileHandle>().as_mut() }) else {
        return SQLITE_IOERR_CLOSE;
    };
    let pointer = std::mem::replace(&mut file.state, ptr::null_mut());
    file.base.pMethods = ptr::null();
    if pointer.is_null() {
        return SQLITE_OK;
    }
    // SAFETY: the pointer is the unique Box installed by xOpen and was removed exactly once.
    let owned = unsafe { Box::from_raw(pointer) };
    let Some(registration) = (unsafe { owned.registration.as_ref() }) else {
        return SQLITE_IOERR_CLOSE;
    };
    registration.run(SQLITE_IOERR_CLOSE, || {
        drop(owned);
        Ok(SQLITE_OK)
    })
}
