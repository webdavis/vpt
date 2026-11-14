use crate::RootDir;
use crate::database::tests::owner::open_callback_file;
use crate::database::{AccessPolicy, FileRole, io::FileHandle, registration::Registration};
use rusqlite::ffi::*;
use std::{ffi::c_void, path::Path, ptr};

struct Callbacks {
    registration: Registration,
    file: Box<FileHandle>,
}

impl Callbacks {
    fn open(root: &RootDir, policy: AccessPolicy) -> Self {
        let registration =
            Registration::new(root.try_clone().unwrap(), Path::new("vpt.db"), policy).unwrap();
        let mode = if policy == AccessPolicy::WritableLedger {
            SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        } else {
            SQLITE_OPEN_READONLY
        };
        let (code, file) = open_callback_file(
            &registration,
            registration.name(),
            SQLITE_OPEN_MAIN_DB | mode,
        );
        assert_eq!(code, SQLITE_OK);
        Self { registration, file }
    }

    fn map(&mut self, extend: i32) -> (i32, *mut c_void) {
        let mut address = ptr::dangling_mut();
        // SAFETY: live registration, file and writable pointer output are retained for the call.
        let code = unsafe {
            (*self.file.base.pMethods).xShmMap.unwrap()(
                &mut self.file.base,
                0,
                32768,
                extend,
                &mut address,
            )
        };
        (code, address)
    }

    fn lock(&mut self, flags: i32) -> i32 {
        // SAFETY: file state remains live until this owner's Drop.
        unsafe { (*self.file.base.pMethods).xShmLock.unwrap()(&mut self.file.base, 0, 1, flags) }
    }

    fn unmap(&mut self, delete: bool) -> i32 {
        // SAFETY: this connection owns the mappings released by its callback.
        unsafe {
            (*self.file.base.pMethods).xShmUnmap.unwrap()(&mut self.file.base, i32::from(delete))
        }
    }
}

impl Drop for Callbacks {
    fn drop(&mut self) {
        assert_eq!(
            // SAFETY: closing once consumes the owned state before the registration drops.
            unsafe { (*self.file.base.pMethods).xClose.unwrap()(&mut self.file.base) },
            SQLITE_OK
        );
    }
}

pub(super) fn run(scenario: &str, root: RootDir) {
    let mut writer = Callbacks::open(&root, AccessPolicy::WritableLedger);
    if scenario == "shm_callback_wal_readonly" {
        writer
            .registration
            .state
            .files
            .open(FileRole::Wal, true)
            .unwrap();
        let registration = Registration::new(
            root.try_clone().unwrap(),
            Path::new("vpt.db"),
            AccessPolicy::ObservationalLedger,
        )
        .unwrap();
        let before = crate::database::tests::support::snapshot(root.path());
        let name = std::ffi::CString::new(format!("{}-wal", registration.name().to_str().unwrap()))
            .unwrap();
        let mut file = FileHandle {
            base: sqlite3_file {
                pMethods: ptr::null(),
            },
            state: ptr::null_mut(),
        };
        let mut actual = 0;
        // SAFETY: the callback has owned output storage and a live registration/name.
        let code = unsafe {
            (*registration.vfs()).xOpen.unwrap()(
                registration.vfs(),
                name.as_ptr(),
                &mut file.base,
                SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_WAL,
                &mut actual,
            )
        };
        assert_eq!(code, SQLITE_OK);
        assert_eq!(
            actual & (SQLITE_OPEN_READONLY | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE),
            SQLITE_OPEN_READONLY
        );
        // SAFETY: successful acquisition retains the file through the write-refusal and close probes.
        unsafe {
            assert_eq!(
                (*file.base.pMethods).xWrite.unwrap()(&mut file.base, [42u8].as_ptr().cast(), 1, 0),
                SQLITE_READONLY
            );
            assert_eq!(
                (*file.base.pMethods).xClose.unwrap()(&mut file.base),
                SQLITE_OK
            );
        }
        assert!(registration.take_error().is_none());
        assert_eq!(
            crate::database::tests::support::snapshot(root.path()),
            before
        );
        return;
    }
    if scenario == "shm_callback_null" {
        assert_eq!(
            // SAFETY: a deliberately null required output is refused without dereferencing it.
            unsafe {
                (*writer.file.base.pMethods).xShmMap.unwrap()(
                    &mut writer.file.base,
                    0,
                    32768,
                    0,
                    ptr::null_mut(),
                )
            },
            SQLITE_IOERR_SHMMAP
        );
        assert!(matches!(
            writer.registration.take_error(),
            Some(crate::database::DatabaseError::Io {
                operation: "shared memory map arguments",
                kind: std::io::ErrorKind::InvalidInput
            })
        ));
        return;
    }
    if scenario == "shm_callback_extend" {
        let (code, address) = writer.map(0);
        assert_eq!(code, SQLITE_OK);
        assert!(address.is_null());
        let (code, address) = writer.map(2);
        assert_eq!(code, SQLITE_OK);
        assert!(!address.is_null());
        assert!(writer.registration.take_error().is_none());
        return;
    }
    assert_eq!(writer.map(1).0, SQLITE_OK);
    match scenario {
        "shm_callback_readonly" => {
            let mut reader = Callbacks::open(&root, AccessPolicy::ObservationalLedger);
            let (code, address) = reader.map(0);
            assert_eq!(code, SQLITE_READONLY);
            assert!(!address.is_null());
            assert!(reader.registration.take_error().is_none());
            drop(reader);
            drop(writer);
            let mut reader = Callbacks::open(&root, AccessPolicy::ObservationalLedger);
            let (code, address) = reader.map(0);
            assert_eq!(code, SQLITE_READONLY_CANTINIT);
            assert!(address.is_null());
            assert!(reader.registration.take_error().is_none());
        }
        "shm_callback_slots" => {
            let mut second = Callbacks::open(&root, AccessPolicy::WritableLedger);
            assert_eq!(second.map(0).0, SQLITE_OK);
            assert_eq!(
                writer.lock(SQLITE_SHM_LOCK | SQLITE_SHM_EXCLUSIVE),
                SQLITE_OK
            );
            assert_eq!(
                second.lock(SQLITE_SHM_LOCK | SQLITE_SHM_EXCLUSIVE),
                SQLITE_BUSY
            );
            assert!(second.registration.take_error().is_none());
            assert_eq!(second.unmap(false), SQLITE_OK);
            let fd = writer
                .registration
                .state
                .files
                .open(FileRole::SharedMemory, false)
                .unwrap();
            super::busy(crate::database::locks::set_record_lock(
                &fd,
                120,
                1,
                crate::database::locks::LockKind::Exclusive,
            ));
            assert_eq!(writer.unmap(false), SQLITE_OK);
            crate::database::locks::set_record_lock(
                &fd,
                120,
                9,
                crate::database::locks::LockKind::Exclusive,
            )
            .unwrap();
        }
        "shm_callback_barrier" => {
            // SAFETY: the void callback operates on this live owner.
            unsafe {
                (*writer.file.base.pMethods).xShmBarrier.unwrap()(&mut writer.file.base);
            }
            assert!(writer.registration.take_error().is_none());
            assert_eq!(writer.map(0).0, SQLITE_OK);
        }
        "shm_callback_persistent" => {
            let mut persistent = 1;
            assert_eq!(
                // SAFETY: file control receives a valid mutable integer.
                unsafe {
                    (*writer.file.base.pMethods).xFileControl.unwrap()(
                        &mut writer.file.base,
                        SQLITE_FCNTL_PERSIST_WAL,
                        (&mut persistent as *mut i32).cast(),
                    )
                },
                SQLITE_OK
            );
            assert_eq!(writer.unmap(true), SQLITE_OK);
            assert!(root.path().join("vpt.db-shm").exists());
        }
        _ => panic!("unknown callback fixture"),
    }
}
