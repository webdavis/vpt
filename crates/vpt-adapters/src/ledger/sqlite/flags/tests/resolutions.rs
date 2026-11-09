use crate::ledger::contract::{
    engine_outputs::committed,
    flags::{accepted, save, seed},
};
use crate::ledger::sqlite::{SqliteLedger, map};
use vpt_application::ports::*;
use vpt_domain::{
    review::{Resolution, ResolutionKind},
    run::{HistoryFact, WorkOutcome},
    time::UtcInstant,
};

#[test]
fn resolved_counts_and_corrections_survive_observational_reopening() {
    crate::ledger::sqlite::tests::supervised("ledger_flags_applied_resolutions");
}
#[test]
fn invalid_saved_corrections_are_refused_without_private_text() {
    crate::ledger::sqlite::tests::supervised("ledger_flags_bad_corrections");
}

pub(super) fn run(scenario: &str, root: &crate::RootDir) -> bool {
    if !matches!(
        scenario,
        "ledger_flags_applied_resolutions" | "ledger_flags_bad_corrections"
    ) {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let run = seed(&ledger);
    committed(save(&ledger, run, accepted()));
    let lexical = crate::lexical_flag_id(&accepted().recording, &accepted().uncertainty.lexical[0]);
    let diagnostic =
        crate::diagnostic_flag_id(&accepted().recording, accepted().uncertainty.diagnostics[0]);
    if scenario == "ledger_flags_bad_corrections" {
        for (index, text) in [
            "private\0".into(),
            "x".repeat(65_537),
            " ".into(),
            "private correction".into(),
        ]
        .iter()
        .enumerate()
        {
            let flag = if index == 3 { diagnostic } else { lexical };
            ledger.transaction(|connection| {
                connection.execute("UPDATE flags SET state='open', resolution_text=NULL, resolved_at=NULL", []).map_err(map)?;
                connection.execute("UPDATE flags SET state='corrected', resolution_text=?1, resolved_at=60 WHERE identifier=?2", rusqlite::params![text, flag.hex()]).map_err(map)?;
                Ok(())
            }).unwrap();
            let error = ledger.transcript_flags(&accepted().recording).unwrap_err();
            assert_eq!(error, crate::ledger::flags::corrupt());
            assert!(!format!("{error:?}").contains("private"));
        }
        return true;
    }
    for (flag, resolution, kind, counts) in [
        (
            lexical,
            Resolution::Corrected {
                text: "Private correction canary".into(),
                at: UtcInstant { secs: 60 },
            },
            ResolutionKind::Corrected,
            (0, 1),
        ),
        (
            diagnostic,
            Resolution::Dismissed {
                at: UtcInstant { secs: 70 },
            },
            ResolutionKind::Dismissed,
            (0, 0),
        ),
    ] {
        let at = resolution.at().unwrap();
        committed(ledger.commit_step(
            run,
            &WorkDelta {
                resolution: Some(FlagResolutionChange {
                    recording: accepted().recording,
                    flag,
                    expected: Resolution::Open,
                    resolution,
                }),
                ..Default::default()
            },
            &[
                (
                    at,
                    HistoryFact::WorkStep {
                        recording: Some(accepted().recording),
                        outcome: WorkOutcome::Completed,
                    },
                ),
                (
                    at,
                    HistoryFact::FlagResolved {
                        recording: accepted().recording,
                        flag,
                        resolution: kind,
                    },
                ),
            ],
        ));
        let saved: (i64, i64) = ledger
            .read(|connection| {
                connection
                    .query_row(
                        "SELECT open_flags, diagnostics FROM recordings",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(map)
            })
            .unwrap();
        assert_eq!(saved, counts);
    }
    drop(ledger);
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let review = vpt_application::review::review(&observer, &accepted().recording).unwrap();
    assert!(review.flags.is_empty());
    assert!(review.diagnostics.is_empty());
    let flags = observer.transcript_flags(&accepted().recording).unwrap();
    assert!(flags.iter().any(|flag| flag.identifier == lexical
        && flag.resolution
            == Resolution::Corrected {
                text: "Private correction canary".into(),
                at: UtcInstant { secs: 60 }
            }));
    assert_eq!(
        observer.accepted_transcript(&accepted().recording).unwrap(),
        Some(accepted())
    );
    true
}
