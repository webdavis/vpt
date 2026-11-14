use super::callbacks::open;
use crate::RootDir;
use crate::database::registration::Registration;
use crate::database::{AccessPolicy, Database, DatabaseError};
use rusqlite::ffi::*;
use std::ffi::CString;
use std::path::Path;
use std::time::Duration;

pub(in crate::database::tests) fn controls_case(scenario: &str, root: RootDir) {
    if scenario == "owner_readonly" {
        readonly(root);
        return;
    }
    let path = root.path().to_path_buf();
    let registration =
        Registration::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    let name = registration.name();
    let flags = SQLITE_OPEN_MAIN_DB | SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE;
    let (result, mut file) = open(&registration, name, flags);
    assert_eq!(result, SQLITE_OK);
    // SAFETY: the registration and successful file remain live, with valid integer output storage.
    unsafe {
        let methods = &*file.base.pMethods;
        match scenario {
            "owner_exclusive_create" => {
                let (result, refused) = open(&registration, name, flags | SQLITE_OPEN_EXCLUSIVE);
                assert_ne!(result, SQLITE_OK);
                assert!(refused.base.pMethods.is_null());
                assert_eq!(std::fs::metadata(path.join("vpt.db")).unwrap().len(), 0);
            }
            "owner_controls" => {
                let mut value = 99i32;
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_LOCKSTATE,
                        (&mut value as *mut i32).cast()
                    ),
                    SQLITE_OK
                );
                assert_eq!(value, SQLITE_LOCK_NONE);
                assert_eq!(
                    methods.xLock.unwrap()(&mut file.base, SQLITE_LOCK_SHARED),
                    SQLITE_OK
                );
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_LOCKSTATE,
                        (&mut value as *mut i32).cast()
                    ),
                    SQLITE_OK
                );
                assert_eq!(value, SQLITE_LOCK_SHARED);
                value = 1;
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_PERSIST_WAL,
                        (&mut value as *mut i32).cast()
                    ),
                    SQLITE_OK
                );
                value = -1;
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_PERSIST_WAL,
                        (&mut value as *mut i32).cast()
                    ),
                    SQLITE_OK
                );
                assert_eq!(value, 1);
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_SIZE_HINT,
                        std::ptr::null_mut()
                    ),
                    SQLITE_NOTFOUND
                );
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_HAS_MOVED,
                        (&mut value as *mut i32).cast()
                    ),
                    SQLITE_OK
                );
                assert_eq!(value, 0);
                std::fs::rename(path.join("vpt.db"), path.join("moved.db")).unwrap();
                std::fs::write(path.join("vpt.db"), b"replacement").unwrap();
                assert_eq!(
                    methods.xFileControl.unwrap()(
                        &mut file.base,
                        SQLITE_FCNTL_HAS_MOVED,
                        (&mut value as *mut i32).cast()
                    ),
                    SQLITE_OK
                );
                assert_eq!(value, 1);
                assert_eq!(
                    methods.xUnlock.unwrap()(&mut file.base, SQLITE_LOCK_NONE),
                    SQLITE_OK
                );
            }
            "owner_path_access" => {
                let vfs = registration.vfs();
                let mut output = [0x55i8; 128];
                assert_ne!(
                    (*vfs).xFullPathname.unwrap()(vfs, name.as_ptr(), 1, output.as_mut_ptr()),
                    SQLITE_OK
                );
                assert_eq!(output[0], 0x55);
                registration.take_error();
                assert_eq!(
                    (*vfs).xFullPathname.unwrap()(vfs, name.as_ptr(), 128, output.as_mut_ptr()),
                    SQLITE_OK
                );
                assert_eq!(std::ffi::CStr::from_ptr(output.as_ptr()), name);
                let target = path.parent().unwrap().join("target");
                std::fs::write(&target, b"outside").unwrap();
                std::os::unix::fs::symlink(&target, path.join("vpt.db-journal")).unwrap();
                let journal = CString::new(format!("{}-journal", name.to_str().unwrap())).unwrap();
                let mut value = 0;
                assert_ne!(
                    (*vfs).xAccess.unwrap()(
                        vfs,
                        journal.as_ptr(),
                        SQLITE_ACCESS_EXISTS,
                        &mut value
                    ),
                    SQLITE_OK
                );
                assert!(
                    registration
                        .take_error()
                        .is_some_and(|error| error.is_containment())
                );
                assert_eq!(std::fs::read(target).unwrap(), b"outside");
            }
            _ => panic!("unknown control fixture"),
        }
        assert_eq!(methods.xClose.unwrap()(&mut file.base), SQLITE_OK);
    }
}

fn readonly(root: RootDir) {
    let path = root.path().to_path_buf();
    let mut writer = Database::open(
        root.try_clone().unwrap(),
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    writer
        .with_connection::<_, DatabaseError>(|connection| {
            connection.execute_batch("CREATE TABLE item(value); INSERT INTO item VALUES(7)")?;
            let mut persistent = -1i32;
            assert_eq!(
                // SAFETY: this test queries the live main file's integer control.
                unsafe {
                    sqlite3_file_control(
                        connection.handle(),
                        c"main".as_ptr(),
                        SQLITE_FCNTL_PERSIST_WAL,
                        (&mut persistent as *mut i32).cast(),
                    )
                },
                SQLITE_OK
            );
            assert_eq!(persistent, 1);
            Ok(())
        })
        .unwrap();
    writer.try_close().unwrap();
    let before = super::super::support::snapshot(&path);
    for policy in [
        AccessPolicy::ObservationalLedger,
        AccessPolicy::RefreshedTitleCopy,
        AccessPolicy::ExistingTitleCopy,
    ] {
        let mut db = Database::open(
            root.try_clone().unwrap(),
            Path::new("vpt.db"),
            policy,
            Duration::from_millis(20),
        )
        .unwrap();
        db.with_connection::<_, DatabaseError>(|connection| {
            assert!(
                connection
                    .db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)?
            );
            assert_eq!(
                connection.query_row("SELECT value FROM item", [], |row| row.get::<_, i64>(0))?,
                7
            );
            assert!(
                connection
                    .execute_batch("INSERT INTO item VALUES(8)")
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
        db.try_close().unwrap();
        assert_eq!(super::super::support::snapshot(&path), before);
    }
}
