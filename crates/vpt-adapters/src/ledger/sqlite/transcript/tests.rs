macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!(
                "ledger_contract_transcript_",
                stringify!($name)
            ));
        }
    };
}
crate::ledger::contract::transcript::cases!(case);

#[test]
fn reopened_and_observational_ledgers_read_accepted_state() {
    super::super::tests::supervised("ledger_transcript_reopen");
}

#[test]
fn corrupt_saved_transcripts_are_refused_without_private_text() {
    super::super::tests::supervised("ledger_transcript_corrupt");
}

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    use super::super::{SqliteLedger, map};
    use crate::ledger::contract::transcript::{
        accepted_transcript_and_history_commit_together, fixture,
    };
    use vpt_application::ports::{HistoryWrite, RunHistory, TranscriptLedger};
    let ledger = SqliteLedger::open(root).unwrap();
    accepted_transcript_and_history_commit_together(&ledger);
    if scenario == "ledger_transcript_reopen" {
        drop(ledger);
        let reopened = SqliteLedger::open(root).unwrap();
        assert_eq!(
            reopened.accepted_transcript(&fixture().recording).unwrap(),
            Some(fixture())
        );
        drop(reopened);
        let observed = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(
            observed.accepted_transcript(&fixture().recording).unwrap(),
            Some(fixture())
        );
        assert!(matches!(
            observed.begin_run(vpt_domain::run::RunKind::Transcribe, fixture().accepted_at),
            HistoryWrite::Unconfirmed(_)
        ));
    } else {
        for body in ["private-transcript-canary", "{\"version\":999}"] {
            ledger
                .transaction(|connection| {
                    connection
                        .execute("UPDATE transcripts SET body = ?1", [body])
                        .map_err(map)?;
                    Ok(())
                })
                .unwrap();
            let error = ledger
                .accepted_transcript(&fixture().recording)
                .unwrap_err();
            assert_eq!(error, crate::ledger::transcript::corrupt());
            assert!(!format!("{error:?}").contains("private-transcript-canary"));
        }
    }
}
