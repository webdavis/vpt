use super::files::DatabaseFiles;
use super::{DatabaseError, FileRole, invalid};
use rusqlite::ffi::*;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::ffi::{CStr, CString};
use std::marker::PhantomPinned;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) struct State {
    pub(super) vfs: UnsafeCell<sqlite3_vfs>,
    pub(super) name: CString,
    pub(super) files: std::rc::Rc<DatabaseFiles>,
    pub(super) native: *mut sqlite3_vfs,
    pub(super) error: RefCell<Option<DatabaseError>>,
    pub(super) poisoned: Cell<bool>,
    pub(super) _pinned: PhantomPinned,
}

impl State {
    pub(super) fn run(
        &self,
        failure: i32,
        operation: impl FnOnce() -> Result<i32, DatabaseError>,
    ) -> i32 {
        let result = if self.poisoned.get() {
            Err(invalid("poisoned callback"))
        } else {
            match catch_unwind(AssertUnwindSafe(operation)) {
                Ok(result) => result,
                Err(_) => {
                    self.poisoned.set(true);
                    Err(invalid("callback panic"))
                }
            }
        };
        match result {
            Ok(code) => code,
            Err(error) => {
                if let DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(code, _)) = &error
                    && (code.code == rusqlite::ErrorCode::DatabaseBusy
                        || code.code == rusqlite::ErrorCode::ReadOnly
                        || code.extended_code == SQLITE_IOERR_SHORT_READ)
                {
                    return code.extended_code;
                }
                let mut latch = self.error.borrow_mut();
                if latch.is_none() || error.is_containment() {
                    *latch = Some(error);
                }
                failure
            }
        }
    }

    pub(super) fn role(&self, name: &CStr) -> Result<FileRole, DatabaseError> {
        let suffix = name
            .to_bytes()
            .strip_prefix(self.name.as_bytes())
            .ok_or_else(|| invalid("filename"))?;
        match suffix {
            b"" => Ok(FileRole::Database),
            b"-journal" => Ok(FileRole::Journal),
            b"-wal" => Ok(FileRole::Wal),
            b"-shm" => Ok(FileRole::SharedMemory),
            _ => Err(invalid("filename")),
        }
    }
}

pub(super) unsafe fn from_vfs<'a>(vfs: *mut sqlite3_vfs) -> Option<&'a State> {
    // SAFETY: SQLite passes the registered table; its pAppData points at pinned State until close.
    unsafe { vfs.as_ref()?.pAppData.cast::<State>().as_ref() }
}
