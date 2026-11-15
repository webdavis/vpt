use super::super::{SCHEMA_VERSION, SqliteLedger, map};
use crate::RootDir;
use vpt_application::ports::{LedgerCommit, RecordingLedger};

#[test]
fn a_schema_nineteen_ledger_upgrades_to_an_identity_and_keeps_every_row() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let recording = crate::ledger::contract::record(19);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    ledger
        .transaction(|c| {
            c.execute_batch(
                "DROP TABLE ledger_writes; DROP TABLE ledger_identity; PRAGMA user_version = 19;",
            )
            .map_err(map)
        })
        .unwrap();
    drop(ledger);
    let older = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(older.schema_version().unwrap(), 19);
    assert_eq!(older.identity().unwrap(), None);
    drop(older);
    let upgraded = SqliteLedger::open(&root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(SCHEMA_VERSION, 20);
    assert_eq!(upgraded.recordings().unwrap(), vec![recording]);
    let identity = upgraded.identity().unwrap().unwrap();
    assert_eq!(identity.database_id.len(), 32);
    assert_eq!(
        (identity.write_counter, identity.recent_tokens.len()),
        (0, 0)
    );
    assert_eq!(identity.latest(), None);
}

#[test]
fn each_stamp_raises_the_counter_and_records_its_token() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let marks: Vec<_> = (0..3).map(|_| ledger.stamp().unwrap()).collect();
    let identity = ledger.identity().unwrap().unwrap();
    assert_eq!(identity.write_counter, 3);
    assert_eq!(identity.latest(), Some(marks[2].clone()));
    let tokens: Vec<_> = marks.iter().map(|mark| mark.write_token.clone()).collect();
    assert_eq!(identity.recent_tokens, tokens);
    assert!(
        marks
            .iter()
            .all(|mark| mark.database_id == identity.database_id)
    );
    let other = crate::test_temp::tempdir().unwrap();
    let elsewhere = SqliteLedger::open(&RootDir::open(other.path()).unwrap()).unwrap();
    assert_ne!(
        elsewhere.identity().unwrap().unwrap().database_id,
        identity.database_id
    );
}

#[test]
fn the_ledger_keeps_ten_thousand_tokens_and_the_next_write_drops_the_oldest() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    ledger
        .transaction(|c| {
            c.execute_batch(
                "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 9999)
                 INSERT INTO ledger_writes (write_counter, token)
                 SELECT i, printf('%032d', i) FROM n;
                 UPDATE ledger_identity SET write_counter = 9999 WHERE id = 1;",
            )
            .map_err(map)
        })
        .unwrap();
    let first = format!("{:032}", 1);
    ledger.stamp().unwrap();
    let full = ledger.identity().unwrap().unwrap().recent_tokens;
    assert_eq!(full.len(), 10_000);
    assert_eq!(full[0], first);
    ledger.stamp().unwrap();
    let trimmed = ledger.identity().unwrap().unwrap().recent_tokens;
    assert_eq!(trimmed.len(), 10_000);
    assert_eq!(trimmed[0], format!("{:032}", 2));
}
