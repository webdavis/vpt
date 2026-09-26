use super::wal::{initialize, open, phase, sql, supervised, values};
use crate::RootDir;
use crate::database::{AccessPolicy, DatabaseError, faults::Fault};

#[test]
fn failed_wal_write_never_commits_a_partial_pair() {
    supervised("failure_wal_write");
}
#[test]
fn failed_wal_partial_write_never_commits_a_partial_pair() {
    supervised("failure_wal_partial");
}
#[test]
fn failed_wal_sync_never_reports_a_successful_commit() {
    supervised("failure_wal_sync");
}
#[test]
fn failed_journal_directory_sync_preserves_atomicity() {
    supervised("failure_directory");
}
#[test]
fn failed_journal_finalization_truncate_preserves_atomicity() {
    supervised("failure_truncate");
}
#[test]
fn first_wal_read_consumes_a_mapping_failure_and_can_reopen() {
    supervised("failure_map");
}

pub(super) fn run(scenario: &str, root: RootDir) {
    phase("seed-open");
    let mut db = open(&root, AccessPolicy::WritableLedger);
    if scenario == "failure_map" {
        initialize(&mut db);
        let before = super::support::snapshot(root.path());
        let mut reader = open(&root, AccessPolicy::ObservationalLedger);
        reader.fail_next(Fault::Map);
        phase("transaction-start");
        let result = values(&mut reader);
        phase("transaction-returned");
        assert!(reader.consumed());
        assert!(
            matches!(
                result,
                Err(DatabaseError::Io {
                    operation: "map",
                    ..
                })
            ),
            "{result:?}"
        );
        phase("close-start");
        reader.try_close().unwrap();
        assert_eq!(super::support::snapshot(root.path()), before);
        phase("reopen-start");
        let mut reader = open(&root, AccessPolicy::ObservationalLedger);
        assert_eq!(values(&mut reader).unwrap(), [1]);
        reader.try_close().unwrap();
        assert_eq!(super::support::snapshot(root.path()), before);
        db.try_close().unwrap();
        phase("done");
        return;
    }
    if scenario.starts_with("failure_wal") {
        initialize(&mut db);
    } else {
        sql(
            &mut db,
            "PRAGMA journal_mode=TRUNCATE; CREATE TABLE item(value); INSERT INTO item VALUES(1)",
        )
        .unwrap();
    }
    if scenario == "failure_directory" {
        sql(
            &mut db,
            "PRAGMA journal_mode=DELETE; INSERT INTO item VALUES(0)",
        )
        .unwrap();
    }
    phase("seed-ready");
    let before = values(&mut db).unwrap();
    match scenario {
        "failure_wal_write" => db.fail_next(Fault::Write),
        "failure_wal_partial" => db.short_write_next(3),
        "failure_wal_sync" => db.fail_next(Fault::FileSync),
        "failure_directory" => db.fail_next(Fault::DirectorySync),
        "failure_truncate" => db.fail_next(Fault::Truncate),
        _ => panic!("unknown failure scenario"),
    }
    phase("transaction-start");
    let result = sql(
        &mut db,
        "BEGIN IMMEDIATE; INSERT INTO item VALUES(2),(3); COMMIT",
    );
    phase("transaction-returned");
    assert!(db.consumed(), "fault must reach its operation");
    assert!(
        matches!(result, Err(DatabaseError::Io { .. })),
        "{result:?}"
    );
    phase("close-start");
    db.try_close().unwrap();
    phase("reopen-start");
    let mut db = open(&root, AccessPolicy::WritableLedger);
    phase("recovery-read");
    let after = values(&mut db).unwrap();
    let mut complete = before.clone();
    complete.extend([2, 3]);
    assert!(
        after == before || after == complete,
        "partial transaction: {after:?}"
    );
    phase("reuse-start");
    sql(&mut db, "BEGIN; INSERT INTO item VALUES(4),(5); COMMIT").unwrap();
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
