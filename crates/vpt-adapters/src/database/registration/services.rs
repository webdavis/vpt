use super::super::{invalid, state::from_vfs};
use rusqlite::ffi::*;
use std::ffi::c_char;

pub(super) unsafe extern "C" fn randomness(
    vfs: *mut sqlite3_vfs,
    length: i32,
    output: *mut c_char,
) -> i32 {
    // SAFETY: SQLite passes back the table registered with this pinned State, which outlives it.
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return 0;
    };
    state.run(0, || {
        if length < 0 || output.is_null() {
            return Err(invalid("randomness arguments"));
        }
        // SAFETY: the retained native table owns this path-free service; SQLite owns output storage.
        let callback =
            unsafe { (*state.native).xRandomness }.ok_or_else(|| invalid("randomness service"))?;
        // SAFETY: the native service gets its own table and SQLite's length-byte output.
        Ok(unsafe { callback(state.native, length, output) })
    })
}

pub(super) unsafe extern "C" fn sleep(vfs: *mut sqlite3_vfs, microseconds: i32) -> i32 {
    // SAFETY: SQLite passes back the table registered with this pinned State, which outlives it.
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return 0;
    };
    state.run(0, || {
        if microseconds < 0 {
            return Err(invalid("sleep duration"));
        }
        // SAFETY: the retained native table owns this path-free service.
        let callback = unsafe { (*state.native).xSleep }.ok_or_else(|| invalid("sleep service"))?;
        // SAFETY: the native service gets its own table and no pointer.
        Ok(unsafe { callback(state.native, microseconds) })
    })
}

pub(super) unsafe extern "C" fn time(vfs: *mut sqlite3_vfs, output: *mut f64) -> i32 {
    // SAFETY: SQLite passes back the table registered with this pinned State, which outlives it.
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return SQLITE_ERROR;
    };
    state.run(SQLITE_ERROR, || {
        if output.is_null() {
            return Err(invalid("time output"));
        }
        // SAFETY: the retained native table owns this path-free service; SQLite owns output storage.
        let callback =
            unsafe { (*state.native).xCurrentTime }.ok_or_else(|| invalid("time service"))?;
        // SAFETY: the native clock gets its own table and SQLite's non-null output storage.
        let code = unsafe { callback(state.native, output) };
        if code != SQLITE_OK {
            return Err(super::super::sqlite_error(code));
        }
        Ok(code)
    })
}

pub(super) unsafe extern "C" fn time_int64(vfs: *mut sqlite3_vfs, output: *mut i64) -> i32 {
    // SAFETY: SQLite passes back the table registered with this pinned State, which outlives it.
    let Some(state) = (unsafe { from_vfs(vfs) }) else {
        return SQLITE_ERROR;
    };
    state.run(SQLITE_ERROR, || {
        if output.is_null() {
            return Err(invalid("time output"));
        }
        // SAFETY: the retained native table owns this path-free service; SQLite owns output storage.
        let callback =
            unsafe { (*state.native).xCurrentTimeInt64 }.ok_or_else(|| invalid("time service"))?;
        // SAFETY: the native clock gets its own table and SQLite's non-null output storage.
        let code = unsafe { callback(state.native, output) };
        if code != SQLITE_OK {
            return Err(super::super::sqlite_error(code));
        }
        Ok(code)
    })
}
