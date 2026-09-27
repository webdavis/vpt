use super::{initialize, open, phase, sql, values};
use crate::RootDir;
use crate::database::tests::support::snapshot;
use crate::database::{AccessPolicy, DatabaseError};
use std::path::Path;
use std::time::{Duration, Instant};

fn observation(root: &RootDir, scenario: &str) {
    let mut writer = open(root, AccessPolicy::WritableLedger);
    initialize(&mut writer);
    if matches!(scenario, "wal_closed_observation" | "wal_heap_observation") {
        writer
            .with_connection::<_, DatabaseError>(|c| {
                c.set_db_config(
                    rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
                    scenario == "wal_heap_observation",
                )?;
                Ok(())
            })
            .unwrap();
    }
    if scenario != "wal_live_observation" {
        writer.try_close().unwrap();
    }
    if scenario == "wal_missing_observation" {
        std::fs::rename(
            root.path().join("vpt.db-shm"),
            root.path().parent().unwrap().join("saved-shm"),
        )
        .unwrap();
    }
    let before = snapshot(root.path());
    let mut reader = open(root, AccessPolicy::ObservationalLedger);
    let result = values(&mut reader);
    if scenario == "wal_missing_observation" {
        assert!(result.is_err(), "{result:?}");
    } else {
        assert_eq!(result.unwrap(), [1]);
    }
    reader.try_close().unwrap();
    assert_eq!(snapshot(root.path()), before);
    writer.try_close().unwrap();
}

fn snapshot_reader(root: &RootDir) {
    let mut writer = open(root, AccessPolicy::WritableLedger);
    initialize(&mut writer);
    let mut reader = open(root, AccessPolicy::ObservationalLedger);
    sql(&mut reader, "BEGIN").unwrap();
    assert_eq!(values(&mut reader).unwrap(), [1]);
    sql(&mut writer, "INSERT INTO item VALUES(2)").unwrap();
    assert_eq!(values(&mut reader).unwrap(), [1]);
    sql(&mut reader, "COMMIT").unwrap();
    assert_eq!(values(&mut reader).unwrap(), [1, 2]);
    reader.try_close().unwrap();
    writer.try_close().unwrap();
}

fn native_interop(root: &RootDir, scenario: &str) {
    let mut rooted = open(
        root,
        if scenario == "wal_native_writer" {
            AccessPolicy::ObservationalLedger
        } else {
            AccessPolicy::WritableLedger
        },
    );
    if scenario == "wal_native_writer" {
        assert_eq!(values(&mut rooted).unwrap(), [1]);
    } else {
        let start = Instant::now();
        let result = sql(
            &mut rooted,
            "BEGIN IMMEDIATE; INSERT INTO item VALUES(9); COMMIT",
        );
        assert!(
            matches!(result, Err(DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::DatabaseBusy),
            "{result:?}"
        );
        assert!(start.elapsed() >= Duration::from_millis(20));
        assert!(start.elapsed() < Duration::from_millis(500));
    }
    rooted.try_close().unwrap();
}

fn crash(root: &RootDir, scenario: &str) {
    if scenario == "wal_recovery_observation" {
        phase("observation-start");
        let before = snapshot(root.path());
        let mut reader = open(root, AccessPolicy::ObservationalLedger);
        assert!(values(&mut reader).is_err());
        reader.try_close().unwrap();
        assert_eq!(snapshot(root.path()), before);
    }
    phase("recovery-open");
    let mut db = open(root, AccessPolicy::WritableLedger);
    phase("recovery-read");
    assert_eq!(
        values(&mut db).unwrap(),
        if scenario == "wal_crash_committed" {
            vec![1, 2, 3]
        } else {
            vec![1]
        }
    );
    phase("reuse-start");
    sql(&mut db, "INSERT INTO item VALUES(4)").unwrap();
    db.with_connection::<_, DatabaseError>(|c| {
        assert_eq!(
            c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?,
            "ok"
        );
        Ok(())
    })
    .unwrap();
    phase("close-final");
    db.try_close().unwrap();
    phase("done");
}

pub(in crate::database::tests) fn run(scenario: &str, root: RootDir) {
    match scenario {
        "wal_close_policy" => super::close::policy(&root),
        "wal_close_committed" => super::close::committed(&root),
        "wal_snapshot" => snapshot_reader(&root),
        "wal_live_observation"
        | "wal_closed_observation"
        | "wal_heap_observation"
        | "wal_missing_observation" => observation(&root, scenario),
        "wal_native_writer" | "wal_native_contention" => native_interop(&root, scenario),
        "wal_crash_uncommitted" | "wal_crash_committed" | "wal_recovery_observation" => {
            crash(&root, scenario)
        }
        _ => panic!("unknown WAL fixture"),
    }
}

pub(in crate::database::tests) fn participant(
    scenario: &str,
    root: RootDir,
    ready: fn(),
    native: fn(&Path) -> rusqlite::Connection,
) {
    if scenario.starts_with("native") {
        let c = native(&root.path().join("vpt.db"));
        if scenario == "native_wal_writer" {
            c.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE item(value); INSERT INTO item VALUES(1); BEGIN IMMEDIATE; INSERT INTO item VALUES(2)").unwrap();
            ready();
            c.execute_batch("ROLLBACK").unwrap();
        } else if scenario == "native_reader" {
            c.execute_batch("BEGIN").unwrap();
            assert_eq!(
                c.query_row("SELECT sum(value) FROM item", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            ready();
            assert_eq!(
                c.query_row("SELECT sum(value) FROM item", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            c.execute_batch("COMMIT").unwrap();
            assert_eq!(
                c.query_row("SELECT sum(value) FROM item", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                3
            );
        } else {
            ready();
            assert!(
                matches!(c.execute_batch("BEGIN IMMEDIATE; INSERT INTO item VALUES(9)"), Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::DatabaseBusy)
            );
        }
        c.close().unwrap();
    } else if matches!(scenario, "rooted_snapshot_writer" | "rooted_wal_writer") {
        let mut db = open(&root, AccessPolicy::WritableLedger);
        initialize(&mut db);
        if scenario == "rooted_snapshot_writer" {
            ready();
            sql(&mut db, "INSERT INTO item VALUES(2)").unwrap();
        } else {
            sql(&mut db, "BEGIN IMMEDIATE; INSERT INTO item VALUES(2)").unwrap();
        }
        ready();
        db.try_close().unwrap();
    } else {
        let mut db = open(&root, AccessPolicy::WritableLedger);
        if scenario == "rooted_rollback" {
            sql(&mut db, "PRAGMA cache_size=1; CREATE TABLE item(value); INSERT INTO item VALUES(1); BEGIN IMMEDIATE; INSERT INTO item VALUES(2); CREATE TABLE spill(data); INSERT INTO spill VALUES(zeroblob(65536))").unwrap();
        } else {
            initialize(&mut db);
            sql(&mut db, "BEGIN IMMEDIATE; INSERT INTO item VALUES(2),(3)").unwrap();
            if scenario == "rooted_committed" {
                sql(&mut db, "COMMIT").unwrap();
            }
        }
        ready();
        panic!("crash fixture must be terminated at its SQL milestone");
    }
}
