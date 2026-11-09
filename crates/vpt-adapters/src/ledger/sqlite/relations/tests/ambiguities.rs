use crate::ledger::contract::{
    engine_outputs::committed,
    relations::{ambiguity, ambiguity_delta},
    tags::{facts, seed},
};
use crate::{RootDir, SqliteLedger};
use vpt_application::ports::*;

#[test]
fn ambiguity_ranges_survive_observational_reopening_without_file_changes() {
    super::super::super::tests::supervised("ledger_relations_ambiguity_reopen");
}
macro_rules! corrupt_case {
    ($name:ident, $index:literal) => {
        #[test]
        fn $name() {
            super::super::super::tests::supervised_prepared(
                concat!("ledger_relations_ambiguity_corrupt_", $index),
                |root| {
                    let ledger = SqliteLedger::open(root).unwrap();
                    let run = seed(&ledger);
                    committed(ledger.commit_step(run, &ambiguity_delta(), &facts()));
                    corrupt(&ledger, $index.parse().unwrap());
                },
            );
        }
    };
}
corrupt_case!(invalid_saved_ambiguity_ranges_are_refused, "0");
corrupt_case!(invalid_saved_ambiguity_term_is_refused, "1");
corrupt_case!(
    oversized_saved_ambiguity_ranges_are_refused_before_loading,
    "2"
);
corrupt_case!(
    oversized_saved_ambiguity_term_is_refused_before_loading,
    "3"
);
corrupt_case!(duplicate_normalized_saved_ambiguity_terms_are_refused, "4");
corrupt_case!(blob_saved_ambiguity_terms_are_refused, "5");

pub(super) fn run(scenario: &str, root: &RootDir) -> bool {
    if !scenario.starts_with("ledger_relations_ambiguity_") {
        return false;
    }
    let id = ambiguity_delta().relations.unwrap().recording;
    if scenario.contains("_corrupt_") {
        let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(
            ledger.mention_ambiguities(&id).unwrap_err(),
            LedgerError::Corrupt("invalid saved relations".into())
        );
        return true;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let run = seed(&ledger);
    committed(ledger.commit_step(run, &ambiguity_delta(), &facts()));
    drop(ledger);
    let before = crate::database::snapshot(root.path());
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(ledger.mention_ambiguities(&id).unwrap(), [ambiguity()]);
    assert_eq!(
        ledger
            .note_inputs(&id)
            .unwrap()
            .unwrap()
            .mention_ambiguities,
        [ambiguity()]
    );
    drop(ledger);
    assert_eq!(crate::database::snapshot(root.path()), before);
    true
}
fn corrupt(ledger: &SqliteLedger, index: usize) {
    let statements = [
        "UPDATE mention_ambiguities SET ranges='[[2,1]]'",
        "UPDATE mention_ambiguities SET term='  '",
        "UPDATE mention_ambiguities SET ranges=zeroblob(16777217)",
        "UPDATE mention_ambiguities SET term=zeroblob(262145)",
        "INSERT INTO mention_ambiguities SELECT recording, 'PRIVATE  PERSON', ranges FROM mention_ambiguities",
        "UPDATE mention_ambiguities SET term=CAST(term AS BLOB)",
    ];
    ledger
        .transaction(|connection| {
            connection
                .execute_batch(statements[index])
                .map_err(crate::ledger::sqlite::map)
        })
        .unwrap();
}
