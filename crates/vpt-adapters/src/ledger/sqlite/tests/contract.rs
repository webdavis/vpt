use crate::RootDir;
use crate::ledger::SqliteLedger;
use crate::ledger::contract;
use vpt_application::ports::{LedgerCommit, LedgerError, PublicationJournal, RecordingLedger};

mod run_history_contract {
    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                super::super::supervised(concat!("ledger_contract_history_", stringify!($name)));
            }
        };
    }
    crate::ledger::contract::history_cases!(case);
}

#[test]
fn a_reopened_ledger_preserves_recording_seen_and_publication() {
    super::supervised("ledger_contract_sqlite_reopen");
}

#[test]
fn seen_integer_boundaries_are_checked_without_partial_write() {
    super::supervised("ledger_contract_sqlite_seen_bounds");
}

#[test]
fn duration_integer_boundaries_are_checked_without_partial_write() {
    super::supervised("ledger_contract_sqlite_duration_bounds");
}

#[test]
fn malformed_stored_fields_are_refused_without_source_content() {
    super::supervised("ledger_contract_sqlite_malformed_fields");
}

#[test]
fn negative_stored_size_is_corrupt() {
    super::supervised("ledger_contract_sqlite_negative_size");
}

#[test]
fn unknown_stored_stage_is_corrupt_without_source_content() {
    super::supervised("ledger_contract_sqlite_unknown_stage");
}

#[test]
fn malformed_stored_publication_digest_is_corrupt_without_source_content() {
    super::supervised("ledger_contract_sqlite_bad_publication_digest");
}

#[test]
fn wrong_typed_publication_column_is_corrupt_without_source_content() {
    super::supervised("ledger_contract_sqlite_wrong_type_publication");
}

pub(super) fn run(scenario: &str, root: &RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    match scenario {
        "ledger_contract_sqlite_reopen" => {
            let recording = contract::record(20);
            let seen = contract::seen_row("/vm/Recordings/20.m4a");
            let publication = contract::publication("/h/transcripts/20.md", 20, 20);
            ledger
                .commit(&LedgerCommit {
                    recordings: vec![recording.clone()],
                    seen: vec![seen.clone()],
                    publications: vec![publication.clone()],
                })
                .unwrap();
            drop(ledger);
            let reopened = SqliteLedger::open(root).unwrap();
            assert_eq!(reopened.by_id(&recording.id).unwrap(), Some(recording));
            assert_eq!(reopened.seen(&seen.path).unwrap(), Some(seen));
            assert_eq!(reopened.pending_publications().unwrap(), vec![publication]);
        }
        "ledger_contract_sqlite_seen_bounds" => {
            let mut row = contract::seen_row("/vm/Recordings/bounds.m4a");
            row.size = i64::MAX as u64;
            row.deferred_size = Some(0);
            row.flags = u32::MAX;
            row.deferral_count = u32::MAX;
            ledger.record_seen(&row).unwrap();
            assert_eq!(ledger.seen(&row.path).unwrap(), Some(row));
            let mut oversized = contract::seen_row("/vm/Recordings/overflow.m4a");
            oversized.size = u64::MAX;
            assert!(matches!(
                ledger.record_seen(&oversized),
                Err(LedgerError::Corrupt(_))
            ));
            assert_eq!(ledger.seen(&oversized.path).unwrap(), None);
        }
        "ledger_contract_sqlite_duration_bounds" => {
            let mut lower = contract::record(21);
            lower.duration_secs = 0;
            let mut upper = contract::record(22);
            upper.duration_secs = i64::MAX as u64;
            ledger
                .commit(&LedgerCommit {
                    recordings: vec![lower.clone(), upper.clone()],
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(ledger.by_id(&lower.id).unwrap(), Some(lower));
            assert_eq!(ledger.by_id(&upper.id).unwrap(), Some(upper));
            let mut overflow = contract::record(23);
            overflow.duration_secs = u64::MAX;
            assert!(matches!(
                ledger.commit(&LedgerCommit {
                    recordings: vec![overflow.clone()],
                    ..Default::default()
                }),
                Err(LedgerError::Corrupt(_))
            ));
            assert_eq!(ledger.by_id(&overflow.id).unwrap(), None);
        }
        "ledger_contract_sqlite_malformed_fields" => {
            let canary = "private-source-content-canary";
            ledger
                .transaction(|transaction| {
                    transaction
                        .execute(
                            "INSERT INTO seen (path, file_name, size, mtime_secs, mtime_nanos, flags, first_seen, last_seen, deferral_count) VALUES (?1, 'bad.m4a', ?2, 0, 0, 0, 0, 0, 0)",
                            ["/vm/Recordings/bad.m4a", canary],
                        )
                        .map(|_| ())
                        .map_err(super::map)
                })
                .unwrap();
            let error = ledger
                .seen(std::path::Path::new("/vm/Recordings/bad.m4a"))
                .unwrap_err();
            assert!(matches!(error, LedgerError::Corrupt(_)));
            assert!(!format!("{error:?}").contains(canary));
        }
        "ledger_contract_sqlite_negative_size" => {
            ledger
                .transaction(|transaction| {
                    transaction
                        .execute(
                            "INSERT INTO seen (path, file_name, size, mtime_secs, mtime_nanos, flags, first_seen, last_seen, deferral_count) VALUES ('/vm/Recordings/negative.m4a', 'negative.m4a', -1, 0, 0, 0, 0, 0, 0)",
                            [],
                        )
                        .map(|_| ())
                        .map_err(super::map)
                })
                .unwrap();
            assert!(matches!(
                ledger.seen(std::path::Path::new("/vm/Recordings/negative.m4a")),
                Err(LedgerError::Corrupt(_))
            ));
        }
        "ledger_contract_sqlite_unknown_stage" => {
            let recording = contract::record(24);
            ledger
                .commit(&LedgerCommit {
                    recordings: vec![recording.clone()],
                    ..Default::default()
                })
                .unwrap();
            let canary = "private-stage-content-canary";
            ledger
                .transaction(|transaction| {
                    transaction
                        .execute(
                            "UPDATE recordings SET stage_note = ?1 WHERE id = ?2",
                            [canary, recording.id.as_str()],
                        )
                        .map(|_| ())
                        .map_err(super::map)
                })
                .unwrap();
            let error = ledger.by_id(&recording.id).unwrap_err();
            assert!(matches!(error, LedgerError::Corrupt(_)));
            assert!(!format!("{error:?}").contains(canary));
        }
        "ledger_contract_sqlite_bad_publication_digest" => {
            let entry = contract::publication("/h/transcripts/bad.md", 25, 25);
            ledger
                .commit(&LedgerCommit {
                    publications: vec![entry],
                    ..Default::default()
                })
                .unwrap();
            let canary = "private-digest-content-canary";
            ledger
                .transaction(|transaction| {
                    transaction
                        .execute(
                            "UPDATE dirty_publications SET intended = ?1 WHERE target = '/h/transcripts/bad.md'",
                            [canary],
                        )
                        .map(|_| ())
                        .map_err(super::map)
                })
                .unwrap();
            let error = ledger.pending_publications().unwrap_err();
            assert!(matches!(error, LedgerError::Corrupt(_)));
            assert!(!format!("{error:?}").contains(canary));
        }
        "ledger_contract_sqlite_wrong_type_publication" => {
            let canary = b"private-publication-blob-canary";
            ledger
                .transaction(|transaction| {
                    transaction
                        .execute(
                            "INSERT INTO dirty_publications (target, expected_previous, intended, recorded_at) VALUES ('/h/transcripts/wrong-type.md', NULL, ?1, 42)",
                            rusqlite::params![&canary[..]],
                        )
                        .map_err(super::map)?;
                    let stored_type: String = transaction
                        .query_row(
                            "SELECT typeof(intended) FROM dirty_publications WHERE target = '/h/transcripts/wrong-type.md'",
                            [],
                            |row| row.get(0),
                        )
                        .map_err(super::map)?;
                    assert_eq!(stored_type, "blob");
                    Ok(())
                })
                .unwrap();
            let error = ledger.pending_publications().unwrap_err();
            assert!(matches!(error, LedgerError::Corrupt(_)), "{error:?}");
            assert!(!format!("{error:?}").contains("private-publication-blob-canary"));
        }
        _ => panic!("unknown owned ledger contract scenario"),
    }
}

mod recording_ledger_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                super::super::supervised(concat!("ledger_contract_recording_", stringify!($name)));
            }
        };
    }
    contract::recording_cases!(case);
}

mod publication_journal_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                super::super::supervised(concat!("ledger_contract_journal_", stringify!($name)));
            }
        };
    }
    contract::journal_cases!(case);
}

mod retention_journal_contract {
    use super::*;

    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                super::super::supervised(concat!("ledger_contract_retention_", stringify!($name)));
            }
        };
    }
    contract::retention_cases!(case);
}
