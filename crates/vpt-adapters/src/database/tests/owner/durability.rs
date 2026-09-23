use crate::RootDir;
use crate::database::faults::Fault;
use crate::database::{AccessPolicy, Database, DatabaseError};
use std::path::Path;
use std::time::Duration;

pub(in crate::database::tests) fn transaction_failure(scenario: &str, root: RootDir) {
    let reopen = root.try_clone().unwrap();
    let mut db = Database::open(
        root,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    db.with_connection::<_, DatabaseError>(|connection| {
        connection.execute_batch("CREATE TABLE item(value); INSERT INTO item VALUES(1)")?;
        Ok(())
    })
    .unwrap();
    match scenario {
        "owner_transaction_sync" => db.fail_next(Fault::FileSync),
        "owner_transaction_directory" => db.fail_next(Fault::DirectorySync),
        "owner_transaction_short" => db.short_write_next(3),
        "owner_transaction_write" => db.fail_next(Fault::Write),
        _ => panic!("unknown transaction fixture"),
    }
    let result = db.with_connection::<_, DatabaseError>(|connection| {
        let _ = connection.execute_batch("BEGIN IMMEDIATE; INSERT INTO item VALUES(2); COMMIT");
        Ok(())
    });
    assert!(db.consumed(), "fault must reach its real operation");
    assert!(
        matches!(result, Err(DatabaseError::Io { .. })),
        "{result:?}"
    );
    db.try_close().unwrap();
    let mut db = Database::open(
        reopen,
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
        Duration::from_millis(20),
    )
    .unwrap();
    let values = db
        .with_connection::<_, DatabaseError>(|connection| {
            let mut statement = connection.prepare("SELECT value FROM item ORDER BY value")?;
            Ok(statement
                .query_map([], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?)
        })
        .unwrap();
    assert_eq!(values, [1]);
    db.try_close().unwrap();
}
