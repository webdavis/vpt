mod resolutions;
macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            super::super::tests::supervised(concat!("ledger_contract_flags_", stringify!($name)));
        }
    };
}
crate::ledger::contract::flags::cases!(case);

#[test]
fn unchanged_resolutions_survive_reopen_and_counts_follow_replacement() {
    super::super::tests::supervised("ledger_flags_resolution");
}

#[test]
fn corrupt_flags_refuse_without_private_text() {
    super::super::tests::supervised("ledger_flags_corrupt");
}

pub(crate) fn run(scenario: &str, root: &crate::RootDir) {
    if resolutions::run(scenario, root) {
        return;
    }
    use super::super::{SqliteLedger, map};
    use crate::ledger::contract::flags::{accepted, save, seed};
    use vpt_application::ports::{FlagLedger, HistoryWrite, TranscriptLedger};
    use vpt_domain::{
        review::{FlagContent, Resolution},
        time::UtcInstant,
    };
    let ledger = SqliteLedger::open(root).unwrap();
    let run = seed(&ledger);
    assert!(matches!(
        save(&ledger, run, accepted()),
        HistoryWrite::Committed(_)
    ));
    let identifier =
        crate::lexical_flag_id(&accepted().recording, &accepted().uncertainty.lexical[0]).hex();
    if scenario == "ledger_flags_corrupt" {
        ledger.transaction(|connection| {
            connection.execute("UPDATE flags SET record_text = 'private-flag-canary' WHERE identifier = ?1", [&identifier]).map_err(map)?;
            Ok(())
        }).unwrap();
        for error in [
            ledger.transcript_flags(&accepted().recording).unwrap_err(),
            ledger
                .accepted_transcript(&accepted().recording)
                .unwrap_err(),
        ] {
            assert_eq!(error, crate::ledger::flags::corrupt());
            assert!(!format!("{error:?}").contains("private-flag-canary"));
        }
        return;
    }
    let counts = |ledger: &SqliteLedger| {
        ledger
            .read(|connection| {
                connection
                    .query_row(
                        "SELECT open_flags, diagnostics FROM recordings",
                        [],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .map_err(map)
            })
            .unwrap()
    };
    assert_eq!(counts(&ledger), (1, 1));
    ledger
        .transaction(|connection| {
            connection
                .execute(
                    "UPDATE flags SET state='confirmed', resolved_at=55 WHERE identifier=?1",
                    [&identifier],
                )
                .map_err(map)?;
            Ok(())
        })
        .unwrap();
    let mut changed = accepted();
    changed.transcript.segments[0].words[0].confidence = Some(0.8);
    changed.uncertainty.lexical[0].confidence = Some(0.8);
    assert!(matches!(
        save(&ledger, run, changed.clone()),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(counts(&ledger), (0, 1));
    drop(ledger);
    let observed = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let flags = observed.transcript_flags(&accepted().recording).unwrap();
    assert!(
        flags
            .iter()
            .any(|flag| matches!(flag.content, FlagContent::Lexical(_))
                && flag.resolution
                    == Resolution::Confirmed {
                        at: UtcInstant { secs: 55 }
                    })
    );
    drop(observed);
    let ledger = SqliteLedger::open(root).unwrap();
    changed.transcript.segments[0].words[0].text = "Different".into();
    changed.uncertainty.lexical[0].record_text = "Different".into();
    assert!(matches!(
        save(&ledger, run, changed),
        HistoryWrite::Committed(_)
    ));
    let flags = ledger.transcript_flags(&accepted().recording).unwrap();
    assert!(flags.iter().all(|flag| flag.resolution == Resolution::Open));
    assert!(flags.iter().all(|flag| flag.identifier.hex() != identifier));
    assert_eq!(counts(&ledger), (1, 1));
}
