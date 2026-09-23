use crate::RootDir;
use crate::database::faults::Fault;
use crate::database::{AccessPolicy, Database, DatabaseError};
use rusqlite::ffi::*;
use std::ffi::CStr;
use std::path::Path;
use std::ptr;
use std::time::Duration;

fn database(root: RootDir) -> Database {
    Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap()
}

fn statement(db: &mut Database) -> (*mut sqlite3_stmt, std::ffi::CString) {
    db.with_connection::<_, DatabaseError>(|connection| {
        let mut statement = ptr::null_mut();
        // SAFETY: this supervised test deliberately retains a statement to exercise owner close.
        unsafe {
            assert_eq!(
                sqlite3_prepare_v2(
                    connection.handle(),
                    c"SELECT 1".as_ptr(),
                    -1,
                    &mut statement,
                    ptr::null_mut()
                ),
                SQLITE_OK
            );
            assert!(!statement.is_null());
            let name = CStr::from_ptr(sqlite3_db_filename(connection.handle(), c"main".as_ptr()))
                .to_owned();
            assert!(!sqlite3_vfs_find(name.as_ptr()).is_null());
            Ok((statement, name))
        }
    })
    .unwrap()
}

pub(in crate::database::tests) fn retry_close(root: RootDir) {
    let second = root.try_clone().unwrap();
    let mut db = database(root);
    db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute_batch("CREATE TABLE item(value); BEGIN EXCLUSIVE")?;
        Ok(())
    })
    .unwrap();
    let (statement, name) = statement(&mut db);
    assert!(
        matches!(db.try_close(), Err(DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(error, _)))
        if error.code == rusqlite::ErrorCode::DatabaseBusy)
    );
    // SAFETY: registration and statement must remain live after the failed synchronous close.
    let retained = unsafe {
        let retained = !sqlite3_vfs_find(name.as_ptr()).is_null();
        assert_eq!(sqlite3_finalize(statement), SQLITE_OK);
        retained
    };
    assert_eq!(
        db.with_connection::<_, DatabaseError>(|connection| {
            Ok(connection.query_row("SELECT 7", [], |row| row.get::<_, i64>(0))?)
        })
        .unwrap(),
        7
    );
    db.try_close().unwrap();
    assert!(retained, "busy close must retain its registered filesystem");
    // SAFETY: lookup copies no dangling registration state.
    assert!(unsafe { sqlite3_vfs_find(name.as_ptr()) }.is_null());
    let mut db = database(second);
    db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute_batch("BEGIN EXCLUSIVE; ROLLBACK")?;
        Ok(())
    })
    .unwrap();
    db.try_close().unwrap();
}

pub(in crate::database::tests) fn invalid_drop(root: RootDir, ready: fn()) {
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: only this owned invalid-lifetime child disables its own core dumps.
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_CORE, &limit) }, 0);
    let mut db = database(root);
    let (_statement, _name) = statement(&mut db);
    ready();
    drop(db);
    panic!("invalid owner drop returned");
}

pub(in crate::database::tests) fn error_latches(root: RootDir) {
    let second = root.try_clone().unwrap();
    let mut first = database(root);
    let mut other = database(second);
    first.fail_next(Fault::Write);
    let result = first.with_connection::<_, DatabaseError>(|connection| {
        let _ = connection.execute_batch("CREATE TABLE item(value)");
        assert_eq!(
            other
                .with_connection::<_, DatabaseError>(|connection| {
                    Ok(connection.query_row("SELECT 7", [], |row| row.get::<_, i64>(0))?)
                })
                .unwrap(),
            7
        );
        Ok(())
    });
    assert!(first.consumed());
    assert!(
        matches!(result, Err(DatabaseError::Io { .. })),
        "{result:?}"
    );
    assert_eq!(
        other
            .with_connection::<_, DatabaseError>(|connection| {
                Ok(connection.query_row("SELECT 7", [], |row| row.get::<_, i64>(0))?)
            })
            .unwrap(),
        7
    );
    first.try_close().unwrap();
    other.try_close().unwrap();
}
