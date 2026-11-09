macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!(
                "ledger_contract_engine_outputs_",
                stringify!($name)
            ));
        }
    };
}
crate::ledger::contract::engine_outputs::cases!(case);

#[test]
fn changed_saved_document_is_refused_without_exposing_its_content() {
    super::super::tests::supervised("ledger_engine_output_corrupt");
}

pub(crate) fn corrupt_saved(root: &crate::RootDir) {
    use vpt_application::ports::{EngineOutputLedger, LedgerError};
    let ledger = super::super::SqliteLedger::open(root).unwrap();
    crate::ledger::contract::engine_outputs::raw_bytes_and_publication_commit_with_accepted_state(
        &ledger,
    );
    ledger
        .transaction(|connection| {
            connection
                .execute(
                    "UPDATE engine_outputs SET body = ?1",
                    [b"private corrupt bytes".as_slice()],
                )
                .map_err(super::super::map)?;
            Ok(())
        })
        .unwrap();
    let error = ledger
        .engine_output(&crate::ledger::contract::engine_outputs::artifact().target)
        .unwrap_err();
    assert!(matches!(error, LedgerError::Corrupt(_)));
    assert!(!format!("{error:?}").contains("private corrupt bytes"));
}
