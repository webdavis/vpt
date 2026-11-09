use crate::ledger::contract::{engine_outputs::committed, tags::*};
use crate::{RootDir, SqliteLedger};
use vpt_application::ports::*;

#[test]
fn saved_tags_and_explicit_empty_state_survive_reopening() {
    super::super::super::tests::supervised("ledger_tags_reopen");
}

#[test]
fn failed_sql_history_append_rolls_back_tags_and_stage() {
    super::super::super::tests::supervised("ledger_tags_sql_rollback");
}

macro_rules! corrupt_case {
    ($name:ident, $index:literal) => {
        #[test]
        fn $name() {
            super::super::super::tests::supervised_prepared(
                concat!("ledger_tags_corrupt_", $index),
                |root| {
                    let ledger = SqliteLedger::open(root).unwrap();
                    let run = seed(&ledger);
                    committed(ledger.commit_step(run, &delta(), &facts()));
                    corrupt(&ledger, $index.parse().unwrap());
                },
            );
        }
    };
}
corrupt_case!(invalid_saved_state_is_refused_privately, "0");
corrupt_case!(invalid_saved_provenance_is_refused_privately, "1");
corrupt_case!(oversized_saved_provenance_is_refused_before_loading, "2");
corrupt_case!(oversized_saved_name_is_refused_before_loading, "3");
corrupt_case!(missing_tag_ordinal_is_refused, "4");
corrupt_case!(uninitialized_recording_with_tags_is_refused, "5");
corrupt_case!(invalid_saved_digest_is_refused_privately, "6");
corrupt_case!(normalized_duplicate_tags_are_refused, "7");
corrupt_case!(non_text_provenance_is_refused, "8");

fn delta() -> WorkDelta {
    WorkDelta {
        tags: Some(change()),
        ..Default::default()
    }
}

pub(super) fn run(scenario: &str, root: &RootDir) -> bool {
    if scenario.starts_with("ledger_tags_corrupt_") {
        let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        let error = ledger.tags(&change().recording).unwrap_err();
        assert_eq!(error, LedgerError::Corrupt("invalid saved tags".into()));
        return true;
    }
    if !matches!(scenario, "ledger_tags_reopen" | "ledger_tags_sql_rollback") {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let run = seed(&ledger);
    if scenario == "ledger_tags_sql_rollback" {
        ledger.transaction(|connection| connection.execute_batch("UPDATE recordings SET stage_note = 'succeeded'; CREATE TRIGGER refuse_tag_history BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT, 'fixture'); END;").map_err(crate::ledger::sqlite::map)).unwrap();
        assert!(matches!(
            ledger.commit_step(run, &delta(), &facts()),
            HistoryWrite::Unconfirmed(_)
        ));
        assert_eq!(ledger.tags(&change().recording).unwrap(), None);
        assert_eq!(
            ledger
                .by_id(&change().recording)
                .unwrap()
                .unwrap()
                .stages
                .note,
            StageState::Succeeded
        );
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
        return true;
    }
    committed(ledger.commit_step(run, &delta(), &facts()));
    drop(ledger);
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(
        observer.tags(&change().recording).unwrap(),
        Some(vec![value()])
    );
    drop(observer);
    let ledger = SqliteLedger::open(root).unwrap();
    let mut change = change();
    change.expected = Some(vec![value()]);
    change.tags.clear();
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            tags: Some(change.clone()),
            ..Default::default()
        },
        &facts(),
    ));
    drop(ledger);
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(observer.tags(&change.recording).unwrap(), Some(vec![]));
    assert_eq!(
        observer
            .tags(&crate::ledger::contract::record(82).id)
            .unwrap(),
        None
    );
    true
}

fn corrupt(ledger: &SqliteLedger, index: usize) {
    let statements = [
        "UPDATE tags SET state = 'private-invalid'",
        "UPDATE tags SET provenance = 'private-invalid'",
        "UPDATE tags SET provenance = zeroblob(16777217)",
        "UPDATE tags SET tag = zeroblob(262145)",
        "UPDATE tags SET ordinal = 1",
        "UPDATE recordings SET tags_initialized = 0",
        "UPDATE tags SET provenance = '{\"kind\":\"proposal\",\"by\":\"private\",\"reason\":\"private\",\"input\":\"bad\"}'",
        "INSERT INTO tags (recording, tag, state, provenance, ordinal) SELECT recording, 'private-tag', state, provenance, 1 FROM tags",
        "UPDATE tags SET provenance = CAST(provenance AS BLOB)",
    ];
    ledger
        .transaction(|connection| {
            connection
                .execute_batch("PRAGMA ignore_check_constraints = ON")
                .map_err(crate::ledger::sqlite::map)?;
            connection
                .execute_batch(statements[index])
                .map_err(crate::ledger::sqlite::map)
        })
        .unwrap();
}
