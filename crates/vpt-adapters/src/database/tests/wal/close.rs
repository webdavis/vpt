use super::{open, sql, values};
use crate::RootDir;
use crate::database::tests::support::snapshot;
use crate::database::{AccessPolicy, Database, DatabaseError};
use rusqlite::config::DbConfig;
use rusqlite::ffi::{SQLITE_FCNTL_PERSIST_WAL, SQLITE_OK, sqlite3_file_control};

fn seed(db: &mut Database) {
    sql(
        db,
        "PRAGMA journal_mode=WAL; CREATE TABLE item(value INTEGER); INSERT INTO item VALUES(7)",
    )
    .unwrap();
}

fn companions(root: &RootDir) {
    for name in ["vpt.db-wal", "vpt.db-shm"] {
        assert!(root.path().join(name).is_file(), "missing {name}");
    }
}

pub(super) fn policy(root: &RootDir) {
    let mut db = open(root, AccessPolicy::WritableLedger);
    seed(&mut db);
    let no_checkpoint = db
        .with_connection::<_, DatabaseError>(|connection| {
            assert_eq!(
                connection.pragma_query_value(None, "synchronous", |row| row.get::<_, u32>(0))?,
                2,
                "writable WAL commits require FULL synchronization"
            );
            assert_eq!(
                connection
                    .pragma_query_value(None, "wal_autocheckpoint", |row| row.get::<_, u32>(0))?,
                1000,
                "commit-triggered automatic checkpoints remain enabled"
            );
            let mut persistent = -1i32;
            // SAFETY: this test queries the live main file into valid integer storage.
            assert_eq!(
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
            Ok(connection.db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)?)
        })
        .unwrap();
    db.try_close().unwrap();
    companions(root);
    assert!(no_checkpoint, "writable close must not checkpoint the WAL");
}

pub(super) fn committed(root: &RootDir) {
    let mut writer = open(root, AccessPolicy::WritableLedger);
    seed(&mut writer);
    assert_eq!(values(&mut writer).unwrap(), [7]);
    assert!(
        std::fs::metadata(root.path().join("vpt.db-wal"))
            .unwrap()
            .len()
            > 32
    );
    let main_before_close = std::fs::read(root.path().join("vpt.db")).unwrap();
    writer.try_close().unwrap();
    companions(root);
    let main_after_close = std::fs::read(root.path().join("vpt.db")).unwrap();
    assert!(
        main_after_close == main_before_close,
        "writable close checkpointed committed WAL frames into the main file"
    );

    let before_observation = snapshot(root.path());
    let mut observer = open(root, AccessPolicy::ObservationalLedger);
    assert_eq!(values(&mut observer).unwrap(), [7]);
    observer.try_close().unwrap();
    assert_eq!(snapshot(root.path()), before_observation);

    let mut reopened = open(root, AccessPolicy::WritableLedger);
    assert_eq!(values(&mut reopened).unwrap(), [7]);
    reopened.try_close().unwrap();
    companions(root);
}
