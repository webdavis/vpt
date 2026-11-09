use crate::{
    RootDir,
    ledger::{
        SqliteLedger,
        contract::{engine_outputs::committed, proposals::fixture, record, verification},
    },
};
use vpt_application::ports::*;

pub(super) fn seed(root: &RootDir) -> SqliteLedger {
    let ledger = SqliteLedger::open(root).unwrap();
    let run = fixture::seed(&ledger);
    let change = verification::change(&ledger);
    committed(verification::save(&ledger, run, change));
    ledger
}

pub(super) fn reopen(root: &RootDir) {
    let ledger = seed(root);
    let before = ledger.review_snapshot(&record(81).id).unwrap();
    let counts = ledger
        .read(|connection| {
            connection
                .query_row(
                    "SELECT open_flags, diagnostics FROM recordings",
                    [],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .map_err(super::super::super::map)
        })
        .unwrap();
    assert_eq!(counts, (2, 0));
    drop(ledger);
    let reopened = SqliteLedger::open(root).unwrap();
    assert_eq!(reopened.review_snapshot(&record(81).id).unwrap(), before);
    drop(reopened);
    let snapshot = crate::database::snapshot(root.path());
    let reader = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(reader.review_snapshot(&record(81).id).unwrap(), before);
    assert_eq!(
        reader.note_inputs(&record(81).id).unwrap().unwrap().review,
        before.unwrap()
    );
    drop(reader);
    assert_eq!(crate::database::snapshot(root.path()), snapshot);
}
