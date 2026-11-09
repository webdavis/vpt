use super::registration::Registration;
use super::{AccessPolicy, DatabaseError, invalid, sqlite_error};
use crate::RootDir;
use rusqlite::{Connection, ffi::*};
use std::ffi::{c_char, c_void};
use std::marker::PhantomData;
use std::path::Path;
use std::ptr;
use std::rc::Rc;
use std::time::Duration;
mod identity;

pub(crate) struct Database {
    connection: Option<Connection>,
    registration: Option<Registration>,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl Database {
    pub(super) fn open_registered(
        registration: Registration,
        busy: Duration,
    ) -> Result<Self, DatabaseError> {
        registration.state.files.revalidate()?;
        let policy = registration.state.files.policy();
        let flags = if policy == AccessPolicy::WritableLedger {
            SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        } else {
            SQLITE_OPEN_READONLY
        };
        let mut raw = RawHandle(ptr::null_mut());
        // SAFETY: the guard owns even a failed-open handle; registration outlives that guard.
        let code = unsafe {
            sqlite3_open_v2(
                registration.name().as_ptr(),
                &mut raw.0,
                flags | SQLITE_OPEN_NOMUTEX | SQLITE_OPEN_EXRESCODE,
                registration.name().as_ptr(),
            )
        };
        if code != SQLITE_OK {
            raw.close();
            return Err(registration
                .take_error()
                .unwrap_or_else(|| sqlite_error(code)));
        }
        // SAFETY: successful open created this uniquely owned handle, guarded until adoption succeeds.
        let connection = unsafe { Connection::from_handle_owned(raw.0) }?;
        raw.0 = ptr::null_mut();
        let mut owner = Self {
            connection: Some(connection),
            registration: Some(registration),
            not_send_or_sync: PhantomData,
        };
        owner.with_connection::<_, DatabaseError>(|connection| {
            configure(connection, policy, busy)
        })?;
        Ok(owner)
    }
    pub(crate) fn open(
        root: RootDir,
        leaf: &Path,
        policy: AccessPolicy,
        busy: Duration,
    ) -> Result<Self, DatabaseError> {
        Self::open_registered(Registration::new(root, leaf, policy)?, busy)
    }

    pub(crate) fn with_connection<T, E>(
        &mut self,
        operation: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<DatabaseError>,
    {
        let connection = self
            .connection
            .as_ref()
            .ok_or_else(|| invalid("closed database"))?;
        let registration = self
            .registration
            .as_ref()
            .ok_or_else(|| invalid("closed registration"))?;
        registration.state.files.revalidate()?;
        registration.state.error.borrow_mut().take();
        if registration.state.poisoned.get() {
            return Err(invalid("poisoned callback").into());
        }
        let result = operation(connection);
        if let Some(error) = registration.take_error() {
            return Err(error.into());
        }
        if result.is_ok() {
            registration.state.files.revalidate()?;
        }
        result
    }

    pub(crate) fn try_close(&mut self) -> Result<(), DatabaseError> {
        let Some(connection) = self.connection.take() else {
            return Ok(());
        };
        if let Err((connection, error)) = connection.close() {
            self.connection = Some(connection);
            return Err(self
                .registration
                .as_ref()
                .and_then(Registration::take_error)
                .unwrap_or_else(|| error.into()));
        }
        let error = self
            .registration
            .as_ref()
            .and_then(Registration::take_error);
        self.registration.take();
        error.map_or(Ok(()), Err)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self.try_close();
        if self.connection.is_some() {
            std::process::abort();
        }
    }
}

fn configure(
    connection: &Connection,
    policy: AccessPolicy,
    busy: Duration,
) -> Result<(), DatabaseError> {
    connection.set_db_config(
        rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
        true,
    )?;
    let milliseconds = i32::try_from(busy.as_millis()).map_err(|_| invalid("busy timeout"))?;
    // SAFETY: the owner retains the connection while configuring checked SQLite settings.
    unsafe {
        let handle = connection.handle();
        for code in [
            sqlite3_extended_result_codes(handle, 1),
            sqlite3_busy_timeout(handle, milliseconds),
            sqlite3_set_authorizer(handle, Some(authorize), ptr::null_mut()),
        ] {
            if code != SQLITE_OK {
                return Err(sqlite_error(code));
            }
        }
        if policy == AccessPolicy::WritableLedger {
            let mut persistent = 1i32;
            let code = sqlite3_file_control(
                handle,
                c"main".as_ptr(),
                SQLITE_FCNTL_PERSIST_WAL,
                (&mut persistent as *mut i32).cast(),
            );
            if code != SQLITE_OK {
                return Err(sqlite_error(code));
            }
        }
    }
    connection.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA mmap_size=0;")?;
    Ok(())
}

unsafe extern "C" fn authorize(
    _: *mut c_void,
    action: i32,
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> i32 {
    if action == SQLITE_ATTACH {
        SQLITE_DENY
    } else {
        SQLITE_OK
    }
}

struct RawHandle(*mut sqlite3);

impl RawHandle {
    fn close(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this guard owns the unadopted handle while registration storage remains live.
            if unsafe { sqlite3_close(self.0) } != SQLITE_OK {
                std::process::abort();
            }
            self.0 = ptr::null_mut();
        }
    }
}

impl Drop for RawHandle {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
impl Database {
    pub(super) fn fail_next(&mut self, fault: super::faults::Fault) {
        self.registration.as_mut().unwrap().fail_next(fault);
    }
    pub(super) fn short_write_next(&mut self, bytes: usize) {
        self.registration.as_mut().unwrap().short_write_next(bytes);
    }
    pub(super) fn consumed(&self) -> bool {
        self.registration.as_ref().unwrap().consumed()
    }
}
