use super::super::{database, supervised, version};
use crate::ledger::{
    contract::{engine_outputs::committed, proposals::fixture, verification},
    sqlite::{SqliteLedger, map, migrations},
};
use vpt_application::ports::*;

#[test]
fn version_ten_checks_require_their_saved_window_without_losing_findings_or_history() {
    supervised("ledger_migration_preserve_v10");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    let run = fixture::seed(&ledger);
    let mut change = verification::change(&ledger);
    change.grounding_window_secs = 73;
    committed(verification::save(&ledger, run, change.clone()));
    let at = vpt_domain::time::UtcInstant { secs: 81 };
    let flag = crate::verification_flag_id(&change.recording, &change.findings[0]);
    committed(ledger.commit_step(
        run,
        &WorkDelta {
            resolution: Some(FlagResolutionChange {
                recording: change.recording.clone(),
                flag,
                expected: vpt_domain::review::Resolution::Open,
                resolution: vpt_domain::review::Resolution::Confirmed { at },
            }),
            ..Default::default()
        },
        &[
            fixture::fact(81, vpt_domain::run::WorkOutcome::Completed),
            (
                at,
                vpt_domain::run::HistoryFact::FlagResolved {
                    recording: change.recording.clone(),
                    flag,
                    resolution: vpt_domain::review::ResolutionKind::Confirmed,
                },
            ),
        ],
    ));
    let before = ledger.review_snapshot(&change.recording).unwrap().unwrap();
    let history = ledger.pending_exports().unwrap();
    drop(ledger);
    database(root, |connection| {
        crate::ledger::sqlite::tests::remove_release_extension(connection)?;
        connection
            .execute_batch("DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339;")
            .map_err(map)?;
        connection.execute_batch(
        "ALTER TABLE verified_artifacts DROP COLUMN grounding_window_secs; PRAGMA user_version=10;"
    ).map_err(map)
    })
    .unwrap();
    let bytes = crate::database::snapshot(root.path());
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let stale = observer
        .review_snapshot(&change.recording)
        .unwrap()
        .unwrap();
    assert_eq!(stale.stale_artifacts, vec![NoteStage::Transcript]);
    assert!(stale.verified_claims.is_empty());
    assert_eq!(stale.stale_findings.len(), 1);
    assert!(matches!(
        stale.stale_findings[0].resolution,
        vpt_domain::review::Resolution::Confirmed { .. }
    ));
    assert_eq!(observer.pending_exports().unwrap(), history);
    drop(observer);
    assert_eq!(crate::database::snapshot(root.path()), bytes);
    database(root, |connection| {
        let error = LedgerError::Io("injected window migration failure".into());
        assert_eq!(migrations::migrate(connection, || Err(error.clone())), Err(error));
        assert_eq!(version(connection), 10);
        assert_eq!(connection.query_row("SELECT count(*) FROM pragma_table_info('verified_artifacts') WHERE name='grounding_window_secs'", [], |row| row.get::<_, u32>(0)).unwrap(), 0);
        Ok(())
    }).unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(
        upgraded
            .review_snapshot(&change.recording)
            .unwrap()
            .unwrap(),
        stale
    );
    assert_eq!(upgraded.pending_exports().unwrap(), history);
    upgraded
        .read(|connection| {
            assert_eq!(version(connection), 15);
            assert_eq!(
                connection
                    .query_row("SELECT open_flags FROM recordings", [], |row| row
                        .get::<_, u32>(0))
                    .unwrap(),
                1
            );
            assert!(
                connection
                    .query_row(
                        "SELECT grounding_window_secs FROM verified_artifacts",
                        [],
                        |row| row.get::<_, Option<u32>>(0)
                    )
                    .unwrap()
                    .is_none()
            );
            Ok(())
        })
        .unwrap();
    committed(verification::save(&upgraded, run, change.clone()));
    assert_eq!(
        upgraded
            .review_snapshot(&change.recording)
            .unwrap()
            .unwrap(),
        before
    );
}
