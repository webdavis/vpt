mod dispatch;
mod services;

use super::files::DatabaseFiles;
use super::state::State;
use super::{AccessPolicy, DatabaseError, invalid, sqlite_error};
use crate::RootDir;
use rusqlite::ffi::*;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::ffi::{CStr, CString};
use std::marker::PhantomPinned;
use std::path::Path;
use std::pin::Pin;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_NAME: AtomicU64 = AtomicU64::new(1);

pub(super) struct Registration {
    pub(super) state: Pin<Box<State>>,
}

impl Registration {
    pub(super) fn new(
        root: RootDir,
        leaf: &Path,
        policy: AccessPolicy,
    ) -> Result<Self, DatabaseError> {
        // SAFETY: these process-wide SQLite queries do not alter threading configuration.
        if unsafe { sqlite3_threadsafe() } == 0 {
            return Err(invalid("SQLite threading"));
        }
        let code = unsafe { sqlite3_initialize() };
        if code != SQLITE_OK {
            return Err(sqlite_error(code));
        }
        let number = NEXT_NAME
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| invalid("registration names exhausted"))?;
        let name =
            CString::new(format!("vpt-db-{number}")).map_err(|_| invalid("registration name"))?;
        // SAFETY: SQLite owns the initialized native registration for process lifetime.
        let native = unsafe { sqlite3_vfs_find(ptr::null()) };
        if native.is_null() {
            return Err(invalid("native services"));
        }
        // SAFETY: null optional callbacks and zero integer fields are a valid unregistered table.
        let mut vfs: sqlite3_vfs = unsafe { std::mem::zeroed() };
        vfs.iVersion = 2;
        vfs.szOsFile = i32::try_from(std::mem::size_of::<super::io::FileHandle>())
            .map_err(|_| invalid("file header size"))?;
        vfs.mxPathname = 128;
        vfs.xOpen = Some(dispatch::open);
        vfs.xDelete = Some(dispatch::delete);
        vfs.xAccess = Some(dispatch::access);
        vfs.xFullPathname = Some(dispatch::full_pathname);
        vfs.xRandomness = Some(services::randomness);
        vfs.xSleep = Some(services::sleep);
        vfs.xCurrentTime = Some(services::time);
        vfs.xCurrentTimeInt64 = Some(services::time_int64);
        let state = Box::pin(State {
            vfs: UnsafeCell::new(vfs),
            name,
            files: std::rc::Rc::new(DatabaseFiles::new(root, leaf, policy)?),
            native,
            error: RefCell::new(None),
            poisoned: Cell::new(false),
            _pinned: PhantomPinned,
        });
        let pointer = state.vfs.get();
        // SAFETY: State is pinned before publishing; UnsafeCell permits SQLite's pNext updates.
        unsafe {
            (*pointer).zName = state.name.as_ptr();
            (*pointer).pAppData = (&*state as *const State).cast_mut().cast();
            let code = sqlite3_vfs_register(pointer, 0);
            if code != SQLITE_OK {
                return Err(sqlite_error(code));
            }
        }
        Ok(Self { state })
    }

    pub(super) fn name(&self) -> &CStr {
        &self.state.name
    }
    pub(super) fn vfs(&self) -> *mut sqlite3_vfs {
        self.state.vfs.get()
    }
    pub(super) fn take_error(&self) -> Option<DatabaseError> {
        self.state.error.borrow_mut().take().or_else(|| {
            self.state
                .poisoned
                .get()
                .then(|| invalid("poisoned callback"))
        })
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        // SAFETY: Database closes every dependent handle before dropping this pinned registration.
        if unsafe { sqlite3_vfs_unregister(self.vfs()) } != SQLITE_OK {
            std::process::abort();
        }
    }
}

#[cfg(test)]
impl Registration {
    pub(super) fn fail_next(&mut self, fault: super::faults::Fault) {
        self.state.files.faults.arm(fault);
    }
    pub(super) fn short_write_next(&mut self, bytes: usize) {
        self.state.files.faults.short_write(bytes);
    }
    pub(super) fn consumed(&self) -> bool {
        self.state.files.faults.consumed.get()
    }
    pub(super) fn panic_callback(&self) -> i32 {
        unsafe extern "C" fn callback(vfs: *mut sqlite3_vfs) -> i32 {
            // SAFETY: this test invokes the C boundary with its live registered table.
            let state = unsafe { super::state::from_vfs(vfs) }.unwrap();
            state.run(SQLITE_IOERR, || panic!("owned callback panic"))
        }
        // SAFETY: the registration remains owned until this test callback returns.
        unsafe { callback(self.vfs()) }
    }
    pub(super) fn fail_directory_sync(&mut self) -> std::rc::Rc<std::cell::Cell<bool>> {
        self.fail_next(super::faults::Fault::DirectorySync);
        self.state.files.faults.consumed.clone()
    }
}
