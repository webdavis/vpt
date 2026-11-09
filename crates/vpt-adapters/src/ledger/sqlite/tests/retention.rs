use super::{SqliteLedger, map};
use crate::RootDir;
use crate::ledger::contract;
use rusqlite::types::Value;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use vpt_application::ports::{LedgerError, RetentionIntent, RetentionJournal};
use vpt_domain::time::UtcInstant;

#[test]
fn reopening_preserves_every_pending_intent_field() {
    super::supervised("ledger_contract_sqlite_retention_reopen");
}

#[test]
fn reopening_preserves_completed_intent_exclusion() {
    super::supervised("ledger_contract_sqlite_retention_completion_reopen");
}

#[test]
fn repeated_completion_preserves_the_first_completed_time() {
    super::supervised("ledger_contract_sqlite_retention_first_completion");
}

#[test]
fn unknown_artifact_kind_is_corrupt_without_value() {
    super::supervised("ledger_contract_sqlite_retention_malformed_kind");
}

#[test]
fn invalid_recording_is_corrupt_without_value() {
    super::supervised("ledger_contract_sqlite_retention_malformed_recording");
}

#[test]
fn null_recording_is_corrupt_without_value() {
    super::supervised("ledger_contract_sqlite_retention_malformed_null_recording");
}

#[test]
fn wrong_typed_path_is_corrupt_without_value() {
    super::supervised("ledger_contract_sqlite_retention_malformed_path");
}

#[test]
fn invalid_digest_is_corrupt_without_value() {
    super::supervised("ledger_contract_sqlite_retention_malformed_digest");
}

#[test]
fn wrong_typed_recorded_time_is_corrupt_without_value() {
    super::supervised("ledger_contract_sqlite_retention_malformed_time");
}

#[test]
fn zero_stored_identifier_is_corrupt() {
    super::supervised("ledger_contract_sqlite_retention_bad_id_zero");
}

#[test]
fn negative_stored_identifier_is_corrupt() {
    super::supervised("ledger_contract_sqlite_retention_bad_id_negative");
}

#[test]
fn non_utf8_path_is_refused_without_an_insert() {
    super::supervised("ledger_contract_sqlite_retention_non_utf8");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    let ledger = SqliteLedger::open(root).expect("owned ledger");
    match scenario {
        "ledger_contract_sqlite_retention_reopen" => {
            let first = contract::retention::fixture(20);
            let second = contract::retention::fixture(21);
            let first_id = ledger.record_intent(&first).expect("first");
            let second_id = ledger.record_intent(&second).expect("second");
            drop(ledger);
            let reopened = SqliteLedger::open(root).expect("reopened");
            assert_eq!(
                reopened.pending_intents().expect("pending"),
                vec![recorded(first_id, first), recorded(second_id, second)]
            );
        }
        "ledger_contract_sqlite_retention_completion_reopen" => {
            let first_id = ledger
                .record_intent(&contract::retention::fixture(20))
                .expect("first");
            let second_id = ledger
                .record_intent(&contract::retention::fixture(21))
                .expect("second");
            ledger
                .complete_intent(first_id, UtcInstant { secs: 9_000 })
                .expect("complete");
            drop(ledger);
            let final_open = SqliteLedger::open(root).expect("final reopen");
            assert_eq!(
                final_open.pending_intents().expect("pending"),
                vec![recorded(second_id, contract::retention::fixture(21))]
            );
        }
        "ledger_contract_sqlite_retention_first_completion" => {
            let id = ledger
                .record_intent(&contract::retention::fixture(22))
                .expect("recorded");
            ledger
                .complete_intent(id, UtcInstant { secs: 9_100 })
                .expect("first");
            ledger
                .complete_intent(id, UtcInstant { secs: 9_101 })
                .expect("repeat");
            let completed: Option<i64> = ledger
                .read(|connection| {
                    connection
                        .query_row(
                            "SELECT completed_at FROM retention_intents WHERE id = ?1",
                            [id],
                            |row| row.get(0),
                        )
                        .map_err(map)
                })
                .expect("completed row");
            assert_eq!(completed, Some(9_100));
        }
        s if s.starts_with("ledger_contract_sqlite_retention_malformed_") => malformed(&ledger, s),
        "ledger_contract_sqlite_retention_bad_id_zero" => nonpositive_id(&ledger, 0),
        "ledger_contract_sqlite_retention_bad_id_negative" => nonpositive_id(&ledger, -1),
        "ledger_contract_sqlite_retention_non_utf8" => {
            let mut new = contract::retention::fixture(23);
            new.path = PathBuf::from(OsString::from_vec(b"/store/audio/\xff.m4a".to_vec()));
            let result = ledger.record_intent(&new);
            assert!(matches!(result, Err(LedgerError::Corrupt(_))), "{result:?}");
            assert!(ledger.pending_intents().expect("no insert").is_empty());
        }
        _ => panic!("unknown retention scenario"),
    }
}

fn recorded(id: i64, new: vpt_application::ports::NewIntent) -> RetentionIntent {
    RetentionIntent {
        id,
        kind: new.kind,
        owner: new.owner,
        path: new.path,
        expected: new.expected,
        recorded_at: new.recorded_at,
    }
}

fn malformed(ledger: &SqliteLedger, scenario: &str) {
    let new = contract::retention::fixture(24);
    let valid = [
        Value::Text("audio".into()),
        Value::Text(new.owner.as_str().into()),
        Value::Text(new.path.to_str().expect("fixture path").into()),
        Value::Text(new.expected.hex()),
        Value::Integer(new.recorded_at.secs),
    ];
    let (column, replacement) = match scenario {
        "ledger_contract_sqlite_retention_malformed_kind" => {
            (0, Value::Text("private-kind-canary".into()))
        }
        "ledger_contract_sqlite_retention_malformed_recording" => {
            (1, Value::Text("private-recording-canary".into()))
        }
        "ledger_contract_sqlite_retention_malformed_null_recording" => (1, Value::Null),
        "ledger_contract_sqlite_retention_malformed_path" => {
            (2, Value::Blob(b"private-path-canary".to_vec()))
        }
        "ledger_contract_sqlite_retention_malformed_digest" => {
            (3, Value::Text("private-digest-canary".into()))
        }
        "ledger_contract_sqlite_retention_malformed_time" => {
            (4, Value::Text("private-time-canary".into()))
        }
        _ => panic!("unknown malformed retention scenario"),
    };
    let mut row = valid;
    row[column] = replacement;
    ledger.transaction(|transaction| {
        transaction.execute(
            "INSERT INTO retention_intents (artifact_kind, recording, path, expected, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params_from_iter(row.iter()),
        ).map_err(map)?;
        Ok(())
    }).expect("inject owned malformed row");
    let error = ledger.pending_intents().expect_err("malformed row refused");
    assert!(matches!(error, LedgerError::Corrupt(_)), "{error:?}");
    let message = format!("{error:?}");
    assert!(!message.contains("private-"), "{message}");
}

fn nonpositive_id(ledger: &SqliteLedger, id: i64) {
    let new = contract::retention::fixture(25);
    ledger.transaction(|transaction| {
        transaction.execute(
            "INSERT INTO retention_intents (id, artifact_kind, recording, path, expected, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, "audio", new.owner.as_str(), new.path.to_str().expect("fixture path"), new.expected.hex(), new.recorded_at.secs],
        ).map_err(map)?;
        Ok(())
    }).expect("inject owned identifier");
    let error = ledger
        .pending_intents()
        .expect_err("nonpositive id refused");
    assert!(matches!(error, LedgerError::Corrupt(_)), "{error:?}");
}
