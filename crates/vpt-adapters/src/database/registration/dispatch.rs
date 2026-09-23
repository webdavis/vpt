use super::super::io::{FileHandle, FileState, METHODS};
use super::super::locks::DatabaseLocks;
use super::super::state::from_vfs;
use super::super::{AccessPolicy, FileRole, invalid, sqlite_error};
use rusqlite::ffi::*;
use std::ffi::{CStr, c_char};
use std::ptr;

pub(super) unsafe extern "C" fn open(
    vfs: *mut sqlite3_vfs,
    name: *const c_char,
    file: *mut sqlite3_file,
    flags: i32,
    output: *mut i32,
) -> i32 {
    // SAFETY: SQLite allocates szOsFile bytes; initialize the whole header before any failure.
    if let Some(file) = unsafe { file.cast::<FileHandle>().as_mut() } {
        file.base.pMethods = ptr::null();
        file.state = ptr::null_mut();
    }
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return SQLITE_CANTOPEN;
    };
    state.run(SQLITE_CANTOPEN, || {
        if name.is_null() || file.is_null() {
            return Err(invalid("open arguments"));
        }
        // SAFETY: SQLite supplies a NUL-terminated name and valid header storage.
        let role = state.role(unsafe { CStr::from_ptr(name) })?;
        let mode = flags & (SQLITE_OPEN_READONLY | SQLITE_OPEN_READWRITE);
        let type_flag = match role {
            FileRole::Database => SQLITE_OPEN_MAIN_DB,
            FileRole::Journal => SQLITE_OPEN_MAIN_JOURNAL,
            FileRole::Wal => SQLITE_OPEN_WAL,
            FileRole::SharedMemory => return Err(invalid("open role")),
        };
        let allowed = type_flag
            | SQLITE_OPEN_READONLY
            | SQLITE_OPEN_READWRITE
            | SQLITE_OPEN_CREATE
            | SQLITE_OPEN_EXCLUSIVE;
        if flags & !allowed != 0
            || flags & type_flag == 0
            || !matches!(mode, SQLITE_OPEN_READONLY | SQLITE_OPEN_READWRITE)
            || (flags & SQLITE_OPEN_CREATE != 0 && mode != SQLITE_OPEN_READWRITE)
            || (flags & SQLITE_OPEN_EXCLUSIVE != 0 && flags & SQLITE_OPEN_CREATE == 0)
        {
            return Err(invalid("open flags"));
        }
        let writable = state.files.policy().writable(role);
        if !writable && mode == SQLITE_OPEN_READWRITE {
            return Err(sqlite_error(SQLITE_READONLY));
        }
        let acquired = if flags & SQLITE_OPEN_EXCLUSIVE != 0 {
            state.files.create(role)?
        } else {
            state.files.open(role, flags & SQLITE_OPEN_CREATE != 0)?
        };
        let owned = Box::new(FileState {
            registration: state,
            locks: DatabaseLocks::new(acquired),
            role,
            writable: writable && mode == SQLITE_OPEN_READWRITE,
            persistent_wal: false,
        });
        // SAFETY: the header takes this one Box; xClose drops it before registration is freed.
        unsafe {
            let file = &mut *file.cast::<FileHandle>();
            file.state = Box::into_raw(owned);
            file.base.pMethods = &METHODS;
            if !output.is_null() {
                *output = flags;
            }
        }
        Ok(SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn delete(
    vfs: *mut sqlite3_vfs,
    name: *const c_char,
    sync: i32,
) -> i32 {
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return SQLITE_IOERR_DELETE;
    };
    state.run(SQLITE_IOERR_DELETE, || {
        if name.is_null() {
            return Err(invalid("delete name"));
        }
        // SAFETY: SQLite supplies a NUL-terminated filename.
        let role = state.role(unsafe { CStr::from_ptr(name) })?;
        if state.files.policy() != AccessPolicy::WritableLedger {
            return Err(sqlite_error(SQLITE_READONLY));
        }
        state.files.remove(role, sync != 0)?;
        Ok(SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn access(
    vfs: *mut sqlite3_vfs,
    name: *const c_char,
    flags: i32,
    output: *mut i32,
) -> i32 {
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return SQLITE_IOERR_ACCESS;
    };
    state.run(SQLITE_IOERR_ACCESS, || {
        if name.is_null()
            || output.is_null()
            || !matches!(
                flags,
                SQLITE_ACCESS_EXISTS | SQLITE_ACCESS_READ | SQLITE_ACCESS_READWRITE
            )
        {
            return Err(invalid("access arguments"));
        }
        // SAFETY: SQLite supplies a NUL-terminated name and integer output storage.
        let role = state.role(unsafe { CStr::from_ptr(name) })?;
        let writable = state.files.policy().writable(role);
        let metadata = state
            .files
            .metadata(role, flags == SQLITE_ACCESS_READWRITE && writable)?;
        let exists = metadata.is_some_and(|metadata| {
            if flags == SQLITE_ACCESS_EXISTS {
                metadata.len() != 0
            } else {
                flags != SQLITE_ACCESS_READWRITE || writable
            }
        });
        unsafe {
            *output = i32::from(exists);
        }
        Ok(SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn full_pathname(
    vfs: *mut sqlite3_vfs,
    name: *const c_char,
    length: i32,
    output: *mut c_char,
) -> i32 {
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return SQLITE_CANTOPEN;
    };
    state.run(SQLITE_CANTOPEN, || {
        if name.is_null() || output.is_null() {
            return Err(invalid("pathname arguments"));
        }
        // SAFETY: SQLite supplies a NUL-terminated filename and length bytes of writable storage.
        if state.role(unsafe { CStr::from_ptr(name) })? != FileRole::Database {
            return Err(invalid("main filename"));
        }
        let bytes = state.name.as_bytes_with_nul();
        if usize::try_from(length)
            .ok()
            .is_none_or(|length| length < bytes.len())
        {
            return Err(invalid("pathname length"));
        }
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().cast(), output, bytes.len());
        }
        Ok(SQLITE_OK)
    })
}
