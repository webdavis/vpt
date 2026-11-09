macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!(
                "ledger_contract_proposals_",
                stringify!($name)
            ));
        }
    };
}
crate::ledger::contract::proposals::cases!(case);

#[test]
fn reopened_and_read_only_ledgers_preserve_accepted_proposal_and_grounding_basis() {
    super::super::tests::supervised("ledger_proposals_reopen");
}

#[test]
fn corrupt_saved_proposal_columns_are_refused_without_private_values() {
    super::super::tests::supervised("ledger_proposals_corrupt");
}

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    use crate::ledger::{SqliteLedger, contract::proposals::fixture};
    use vpt_application::ports::*;
    use vpt_domain::run::WorkOutcome;
    let ledger = SqliteLedger::open(root).unwrap();
    let run = fixture::seed(&ledger);
    let accepted = fixture::accepted(&ledger);
    crate::ledger::contract::engine_outputs::committed(ledger.commit_step(
        run,
        &fixture::delta(accepted.clone()),
        &[fixture::fact(70, WorkOutcome::Completed)],
    ));
    if scenario == "ledger_proposals_reopen" {
        drop(ledger);
        let reopened = SqliteLedger::open(root).unwrap();
        assert_eq!(
            reopened.accepted_proposal(&accepted.recording).unwrap(),
            Some(accepted.clone())
        );
        assert_eq!(
            reopened
                .note_inputs(&accepted.recording)
                .unwrap()
                .unwrap()
                .proposal,
            Some(accepted.clone())
        );
        drop(reopened);
        let snapshot = crate::database::snapshot(root.path());
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(
            observer.accepted_proposal(&accepted.recording).unwrap(),
            Some(accepted.clone())
        );
        assert_eq!(
            observer.grounding_basis(&accepted.recording).unwrap(),
            Some(accepted.basis)
        );
        assert_eq!(
            observer
                .note_inputs(&accepted.recording)
                .unwrap()
                .unwrap()
                .proposal,
            Some(accepted.clone())
        );
        drop(observer);
        assert_eq!(crate::database::snapshot(root.path()), snapshot);
    } else {
        let body = crate::ledger::proposal::codec::encode(&accepted).unwrap();
        for sql in [
            "UPDATE proposals SET input_digest='private-invalid-digest'",
            "UPDATE proposals SET body='private-invalid-body'",
            "UPDATE proposals SET accepted_at='private-invalid-time'",
            "UPDATE proposals SET body=zeroblob(16777217)",
        ] {
            ledger
                .transaction(|connection| {
                    connection
                        .execute(
                            "UPDATE proposals SET input_digest=?1,body=?2,accepted_at=?3",
                            rusqlite::params![
                                accepted.input_digest.hex(),
                                body,
                                accepted.accepted_at.secs
                            ],
                        )
                        .map_err(super::super::map)?;
                    connection.execute(sql, []).map_err(super::super::map)?;
                    Ok(())
                })
                .unwrap();
            let error = ledger.accepted_proposal(&accepted.recording).unwrap_err();
            assert!(!format!("{error:?}").contains("private-"));
            let error = ledger
                .note_inputs(&accepted.recording)
                .err()
                .expect("invalid proposals cannot enter a render snapshot");
            assert!(!format!("{error:?}").contains("private-"));
        }
    }
}
