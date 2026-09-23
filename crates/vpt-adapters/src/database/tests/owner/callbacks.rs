use crate::RootDir;
use crate::database::faults::Fault;
use crate::database::io::FileHandle;
use crate::database::registration::Registration;
use crate::database::{AccessPolicy, DatabaseError};
use rusqlite::ffi::*;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

pub(super) fn open(registration: &Registration, name: &CStr, flags: i32) -> (i32, Box<FileHandle>) {
    let mut file = Box::new(FileHandle {
        base: sqlite3_file {
            pMethods: ptr::null(),
        },
        state: ptr::null_mut(),
    });
    let vfs = registration.vfs();
    assert!(
        !vfs.is_null(),
        "registration must supply initialized callbacks"
    );
    // SAFETY: registration and output storage live across the callback; names are NUL-terminated.
    let result = unsafe {
        (*vfs).xOpen.unwrap()(vfs, name.as_ptr(), &mut file.base, flags, ptr::null_mut())
    };
    (result, file)
}

pub(super) fn run(scenario: &str, root: RootDir) {
    let path = root.path().to_path_buf();
    let mut registration =
        Registration::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    let name = registration.name().to_owned();
    let (result, mut file) = open(
        &registration,
        &name,
        SQLITE_OPEN_MAIN_DB | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
    );
    assert_eq!(result, SQLITE_OK);
    // SAFETY: successful xOpen initialized the file's static method table.
    let methods = unsafe { &*file.base.pMethods };
    let vfs = registration.vfs();
    let journal = CString::new(format!("{}-journal", name.to_str().unwrap())).unwrap();
    // SAFETY: each callback receives live registration/file storage and correctly sized owned buffers.
    unsafe {
        match scenario {
            "owner_short_read" => {
                assert_eq!(
                    methods.xWrite.unwrap()(&mut file.base, b"abc".as_ptr().cast(), 3, 0),
                    SQLITE_OK
                );
                let mut bytes = [0x7f; 6];
                assert_eq!(
                    methods.xRead.unwrap()(&mut file.base, bytes.as_mut_ptr().cast(), 6, 1),
                    SQLITE_IOERR_SHORT_READ
                );
                assert_eq!(bytes, [b'b', b'c', 0, 0, 0, 0]);
                assert!(registration.take_error().is_none());
            }
            "owner_partial_write" => {
                registration.short_write_next(2);
                assert_eq!(
                    methods.xWrite.unwrap()(&mut file.base, b"abcdef".as_ptr().cast(), 6, 0),
                    SQLITE_IOERR_WRITE
                );
                assert!(registration.consumed());
                assert_eq!(std::fs::read(path.join("vpt.db")).unwrap(), b"ab");
                assert!(matches!(
                    registration.take_error(),
                    Some(DatabaseError::Io { .. })
                ));
            }
            "owner_invalid_offsets" => {
                let mut bytes = [0; 3];
                assert_eq!(
                    methods.xRead.unwrap()(&mut file.base, bytes.as_mut_ptr().cast(), 3, -1),
                    SQLITE_IOERR_READ
                );
                assert_eq!(
                    methods.xWrite.unwrap()(&mut file.base, bytes.as_ptr().cast(), 3, i64::MAX),
                    SQLITE_IOERR_WRITE
                );
                assert_eq!(
                    methods.xRead.unwrap()(&mut file.base, ptr::null_mut(), 3, 0),
                    SQLITE_IOERR_READ
                );
                assert_eq!(
                    methods.xTruncate.unwrap()(&mut file.base, -1),
                    SQLITE_IOERR_TRUNCATE
                );
                assert_eq!(
                    methods.xFileSize.unwrap()(&mut file.base, ptr::null_mut()),
                    SQLITE_IOERR_FSTAT
                );
                assert_eq!(std::fs::metadata(path.join("vpt.db")).unwrap().len(), 0);
            }
            "owner_unknown_open" => {
                for (name, flags) in [
                    (c"other", SQLITE_OPEN_MAIN_DB),
                    (c"../escape", SQLITE_OPEN_MAIN_DB),
                    (c"vpt-test-unknown", SQLITE_OPEN_MAIN_JOURNAL),
                    (&name, SQLITE_OPEN_TEMP_DB),
                    (&name, SQLITE_OPEN_MAIN_DB | 0x4000_0000),
                ] {
                    let (result, refused) = open(
                        &registration,
                        name,
                        flags | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
                    );
                    assert_ne!(result, SQLITE_OK);
                    assert!(refused.base.pMethods.is_null());
                    assert!(refused.state.is_null());
                }
                let mut refused = FileHandle {
                    base: sqlite3_file {
                        pMethods: ptr::null(),
                    },
                    state: ptr::null_mut(),
                };
                assert_ne!(
                    (*vfs).xOpen.unwrap()(
                        vfs,
                        ptr::null(),
                        &mut refused.base,
                        SQLITE_OPEN_TEMP_DB | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
            }
            "owner_root_switch" => {
                assert_eq!(std::env::current_dir().unwrap(), path.parent().unwrap());
                assert!(
                    name.to_bytes()
                        .iter()
                        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
                );
                let held = path.with_file_name("held");
                std::fs::rename(&path, &held).unwrap();
                std::fs::create_dir(&path).unwrap();
                std::fs::write(path.join("vpt.db-journal"), b"replacement").unwrap();
                let (result, mut journal_file) = open(
                    &registration,
                    &journal,
                    SQLITE_OPEN_MAIN_JOURNAL | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
                );
                assert_eq!(result, SQLITE_OK);
                assert!(held.join("vpt.db-journal").is_file());
                assert_eq!(
                    (*journal_file.base.pMethods).xClose.unwrap()(&mut journal_file.base),
                    SQLITE_OK
                );
                assert_eq!((*vfs).xDelete.unwrap()(vfs, journal.as_ptr(), 1), SQLITE_OK);
                assert!(!held.join("vpt.db-journal").exists());
                assert_eq!(
                    std::fs::read(path.join("vpt.db-journal")).unwrap(),
                    b"replacement"
                );
                assert!(!path.parent().unwrap().join(name.to_str().unwrap()).exists());
            }
            "owner_panic" => {
                assert_eq!(registration.panic_callback(), SQLITE_IOERR);
                assert!(registration.take_error().is_some());
                let mut size = 0;
                assert_eq!(
                    methods.xFileSize.unwrap()(&mut file.base, &mut size),
                    SQLITE_IOERR_FSTAT
                );
            }
            "owner_delete_sync" => {
                let (result, mut journal_file) = open(
                    &registration,
                    &journal,
                    SQLITE_OPEN_MAIN_JOURNAL | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
                );
                assert_eq!(result, SQLITE_OK);
                assert_eq!(
                    (*journal_file.base.pMethods).xClose.unwrap()(&mut journal_file.base),
                    SQLITE_OK
                );
                registration.fail_next(Fault::DirectorySync);
                assert_eq!(
                    (*vfs).xDelete.unwrap()(vfs, journal.as_ptr(), 1),
                    SQLITE_IOERR_DELETE
                );
                assert!(registration.consumed());
                assert!(!path.join("vpt.db-journal").exists());
                assert!(registration.take_error().is_some());
            }
            "owner_faults" => {
                for fault in [Fault::Write, Fault::FileSync, Fault::Truncate] {
                    registration.fail_next(fault);
                    let result = match fault {
                        Fault::Write => {
                            methods.xWrite.unwrap()(&mut file.base, b"abc".as_ptr().cast(), 3, 0)
                        }
                        Fault::FileSync => methods.xSync.unwrap()(&mut file.base, SQLITE_SYNC_FULL),
                        Fault::Truncate => methods.xTruncate.unwrap()(&mut file.base, 10),
                        _ => unreachable!(),
                    };
                    assert_ne!(result, SQLITE_OK);
                    assert!(registration.consumed());
                    assert!(registration.take_error().is_some());
                }
            }
            _ => panic!("unknown owner callback fixture"),
        }
        let result = methods.xClose.unwrap()(&mut file.base);
        assert!(result == SQLITE_OK || scenario == "owner_panic");
        assert!(file.state.is_null());
    }
}
