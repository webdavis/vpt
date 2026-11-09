use crate::{RootDir, ledger::contract::record};
use vpt_application::ports::{NoteInputsLedger, ReviewLedger};

macro_rules! cases {
    ($case:ident) => {
        $case!(input, "input_digest=printf('%064s','private-input')");
        $case!(claims, "claims_digest=printf('%064s','private-claims')");
        $case!(
            transcript,
            "transcript_digest=printf('%064s','private-transcript')"
        );
        $case!(lexical, "lexical_digest=printf('%064s','private-lexical')");
        $case!(time, "checked_at='private-time'");
        $case!(window_text, "grounding_window_secs='private-window'");
        $case!(window_negative, "grounding_window_secs=-1");
        $case!(window_overflow, "grounding_window_secs=4294967296");
    };
}

macro_rules! test {
    ($name:ident, $assignment:literal) => {
        #[test]
        fn $name() {
            super::super::super::tests::supervised(concat!(
                "ledger_verification_basis_",
                stringify!($name)
            ));
        }
    };
}
cases!(test);

pub(super) fn run(case: &str, root: &RootDir) {
    let ledger = super::durability::seed(root);
    macro_rules! dispatch {
        ($name:ident, $assignment:literal) => {
            if case == stringify!($name) {
                ledger
                    .transaction(|connection| {
                        if case.starts_with("window_") {
                            connection
                                .pragma_update(None, "ignore_check_constraints", true)
                                .map_err(super::super::super::map)?;
                        }
                        connection
                            .execute(concat!("UPDATE verified_artifacts SET ", $assignment), [])
                            .map_err(super::super::super::map)?;
                        Ok(())
                    })
                    .unwrap();
                for error in [
                    ledger.review_snapshot(&record(81).id).unwrap_err(),
                    ledger
                        .note_inputs(&record(81).id)
                        .err()
                        .expect("corrupt basis must refuse note inputs"),
                ] {
                    assert!(!format!("{error:?}").contains("private"));
                }
                return;
            }
        };
    }
    cases!(dispatch);
    panic!("unknown verification basis scenario");
}
