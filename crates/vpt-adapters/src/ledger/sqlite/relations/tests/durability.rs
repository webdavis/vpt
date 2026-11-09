use crate::ledger::contract::{
    engine_outputs::committed,
    relations::{delta, values},
    tags::{facts, seed},
};
use crate::{RootDir, SqliteLedger};
use vpt_application::ports::*;

#[test]
fn relations_survive_read_only_reopening_without_changing_database_files() {
    super::super::super::tests::supervised("ledger_relations_reopen");
}
#[test]
fn a_failed_sql_history_append_rolls_back_relations_and_note_state() {
    super::super::super::tests::supervised("ledger_relations_sql_rollback");
}
macro_rules! corrupt_case {
    ($name:ident, $index:literal) => {
        #[test]
        fn $name() {
            super::super::super::tests::supervised_prepared(
                concat!("ledger_relations_corrupt_", $index),
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
corrupt_case!(unknown_saved_relation_kind_is_refused, "0");
corrupt_case!(unknown_saved_relation_state_is_refused, "1");
corrupt_case!(unknown_saved_relation_rule_is_refused, "2");
corrupt_case!(invalid_saved_relation_target_is_refused, "3");
corrupt_case!(invalid_saved_relation_provenance_is_refused, "4");
corrupt_case!(invalid_saved_relation_ranges_are_refused, "5");
corrupt_case!(oversized_saved_relation_fields_are_refused, "6");
corrupt_case!(blob_saved_relation_fields_are_refused, "7");
corrupt_case!(noncanonical_saved_relation_paths_are_refused, "8");
corrupt_case!(nonmanual_supersedes_is_refused, "9");

pub(super) fn run(scenario: &str, root: &RootDir) -> bool {
    if scenario.starts_with("ledger_relations_corrupt_") {
        let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        let id = delta().relations.unwrap().recording;
        assert_eq!(
            ledger.relations(&id).unwrap_err(),
            LedgerError::Corrupt("invalid saved relations".into())
        );
        return true;
    }
    if !matches!(
        scenario,
        "ledger_relations_reopen" | "ledger_relations_sql_rollback"
    ) {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let run = seed(&ledger);
    let id = delta().relations.unwrap().recording;
    if scenario == "ledger_relations_sql_rollback" {
        ledger.transaction(|connection| connection.execute_batch("UPDATE recordings SET stage_note='succeeded'; CREATE TRIGGER refuse_relation_history BEFORE INSERT ON history_records BEGIN SELECT RAISE(ABORT, 'fixture'); END;").map_err(crate::ledger::sqlite::map)).unwrap();
        assert!(matches!(
            ledger.commit_step(run, &delta(), &facts()),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.relations(&id).unwrap().is_empty());
        assert_eq!(
            ledger.by_id(&id).unwrap().unwrap().stages.note,
            StageState::Succeeded
        );
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    } else {
        committed(ledger.commit_step(run, &delta(), &facts()));
        drop(ledger);
        let before = crate::database::snapshot(root.path());
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(observer.relations(&id).unwrap(), values());
        drop(observer);
        assert_eq!(crate::database::snapshot(root.path()), before);
        let ledger = SqliteLedger::open(root).unwrap();
        let mut delta = delta();
        let change = delta.relations.as_mut().unwrap();
        change.expected = values();
        change.relations.clear();
        committed(ledger.commit_step(run, &delta, &facts()));
        drop(ledger);
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert!(observer.relations(&id).unwrap().is_empty());
    }
    true
}
fn corrupt(ledger: &SqliteLedger, index: usize) {
    let statements = [
        "UPDATE relations SET kind='Private-invalid' WHERE kind='mentions'",
        "UPDATE relations SET state='Private-invalid'",
        "UPDATE relations SET rule='Private-invalid' WHERE kind='mentions'",
        "UPDATE relations SET target='Private-invalid' WHERE kind='mentions'",
        "UPDATE relations SET provenance='Private-invalid'",
        "UPDATE relations SET ranges='[[2,1]]' WHERE kind='mentions'",
        "UPDATE relations SET provenance=zeroblob(16777217) WHERE kind='mentions'",
        "UPDATE relations SET provenance=CAST(provenance AS BLOB)",
        "UPDATE relations SET target='/vault/./Private.md' WHERE kind='mentions'",
        "UPDATE relations SET provenance='{\"kind\":\"derived\"}', rule='known_term' WHERE kind='supersedes'",
    ];
    ledger
        .transaction(|connection| {
            connection
                .execute_batch(statements[index])
                .map_err(crate::ledger::sqlite::map)
        })
        .unwrap();
}
