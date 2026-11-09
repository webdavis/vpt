use crate::{RootDir, ledger::contract::record};
use vpt_application::ports::ReviewLedger;

macro_rules! cases {
    ($case:ident) => {
        $case!(artifact, "artifact='private-invalid-stage'");
        $case!(digest, "claim_digest=printf('%064d',0)");
        $case!(negative_index, "claim_index=-1");
        $case!(oversized_index, "claim_index=65536");
        $case!(claim_text, "record_text='private altered claim'");
        $case!(confidence, "confidence=0.5");
        $case!(ranges, "ranges='[]'");
        $case!(alternative, "alternative_text='private unexpected token'");
        $case!(class, "class='private-invalid-class'");
        $case!(
            resolution,
            "resolution_text='private unexpected resolution'"
        );
        $case!(oversized_digest, "claim_digest=zeroblob(65)");
        $case!(oversized_text, "record_text=zeroblob(262145)");
    };
}

macro_rules! test {
    ($name:ident, $assignment:literal) => {
        #[test]
        fn $name() {
            super::super::super::tests::supervised(concat!(
                "ledger_verification_corrupt_",
                stringify!($name)
            ));
        }
    };
}
cases!(test);

pub(super) fn run(case: &str, root: &RootDir) {
    macro_rules! dispatch {
        ($name:ident, $assignment:literal) => {
            if case == stringify!($name) {
                corrupt(
                    root,
                    concat!(
                        "UPDATE flags SET ",
                        $assignment,
                        " WHERE shape='verification'"
                    ),
                );
                return;
            }
        };
    }
    cases!(dispatch);
    panic!("unknown verification corruption scenario");
}

fn corrupt(root: &RootDir, sql: &str) {
    let ledger = super::durability::seed(root);
    ledger
        .transaction(|connection| {
            connection
                .execute(sql, [])
                .map_err(super::super::super::map)?;
            Ok(())
        })
        .unwrap();
    let error = ledger.review_snapshot(&record(81).id).unwrap_err();
    assert!(!format!("{error:?}").contains("private"));
}
