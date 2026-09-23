use super::super::invalid;
use super::run;
use rusqlite::ffi::*;
use std::ffi::c_void;

pub(super) unsafe extern "C" fn lock(file: *mut sqlite3_file, level: i32) -> i32 {
    // SAFETY: run validates file state before invoking its owned lock protocol.
    unsafe {
        run(file, SQLITE_IOERR_LOCK, |file, _| {
            file.locks.lock(level)?;
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn unlock(file: *mut sqlite3_file, level: i32) -> i32 {
    // SAFETY: run validates file state before invoking its owned lock protocol.
    unsafe {
        run(file, SQLITE_IOERR_UNLOCK, |file, _| {
            file.locks.unlock(level)?;
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn reserved(file: *mut sqlite3_file, output: *mut i32) -> i32 {
    // SAFETY: run validates file state; SQLite supplies writable integer output.
    unsafe {
        run(file, SQLITE_IOERR_CHECKRESERVEDLOCK, |file, _| {
            if output.is_null() {
                return Err(invalid("reserved output"));
            }
            *output = i32::from(file.locks.reserved()?);
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn control(
    file: *mut sqlite3_file,
    operation: i32,
    argument: *mut c_void,
) -> i32 {
    // SAFETY: SQLite supplies writable integer storage for the three supported controls.
    unsafe {
        run(file, SQLITE_IOERR, |file, state| {
            if !matches!(
                operation,
                SQLITE_FCNTL_LOCKSTATE | SQLITE_FCNTL_PERSIST_WAL | SQLITE_FCNTL_HAS_MOVED
            ) {
                return Ok(SQLITE_NOTFOUND);
            }
            let argument = argument
                .cast::<i32>()
                .as_mut()
                .ok_or_else(|| invalid("file control argument"))?;
            match operation {
                SQLITE_FCNTL_LOCKSTATE => *argument = file.locks.level(),
                SQLITE_FCNTL_PERSIST_WAL => {
                    if *argument < 0 {
                        *argument = i32::from(file.persistent_wal);
                    } else {
                        file.persistent_wal = *argument != 0;
                    }
                }
                SQLITE_FCNTL_HAS_MOVED => {
                    *argument = i32::from(state.files.moved(file.role, file.locks.file())?)
                }
                _ => unreachable!(),
            }
            Ok(SQLITE_OK)
        })
    }
}

pub(super) unsafe extern "C" fn sector(file: *mut sqlite3_file) -> i32 {
    // SAFETY: run validates file state for this count callback.
    unsafe { run(file, 0, |_, _| Ok(4096)) }
}

pub(super) unsafe extern "C" fn characteristics(file: *mut sqlite3_file) -> i32 {
    // SAFETY: run validates file state for this capability callback.
    unsafe { run(file, 0, |_, _| Ok(0)) }
}
