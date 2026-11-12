use crate::ledger::contract::{
    briefs::fixture::*,
    occasions::{saved, write},
};
use crate::ledger::sqlite::{SqliteLedger, map, migrations};
use vpt_application::ports::*;
use vpt_domain::{occasion::time::OccasionTime, time::UtcInstant};

#[test]
fn saved_brief_reopens_and_rescheduling_preserves_its_frozen_snapshot() {
    super::super::super::tests::supervised("ledger_briefs_reopen");
}
#[test]
fn schema_twelve_migration_preserves_legacy_packs_and_rolls_back_on_failure() {
    super::super::super::tests::supervised("ledger_briefs_migration");
}
#[test]
fn corrupt_saved_brief_columns_are_refused_without_echoing_content() {
    super::super::super::tests::supervised("ledger_briefs_corrupt");
}

pub(super) fn run(scenario: &str, root: &crate::RootDir) -> bool {
    if !matches!(
        scenario,
        "ledger_briefs_reopen" | "ledger_briefs_migration" | "ledger_briefs_corrupt"
    ) {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let (run, row) = occasion(&ledger);
    if scenario == "ledger_briefs_migration" {
        migration(root, ledger, &row);
        return true;
    }
    let artifact = artifact(&row, 10);
    saved(save(&ledger, run, &row, None, &artifact));
    if scenario == "ledger_briefs_corrupt" {
        for statement in [
            "UPDATE owned_briefs SET body='Private replaced body'",
            "UPDATE owned_briefs SET digest='Private invalid digest'",
            "UPDATE owned_briefs SET snapshot_digest=zeroblob(65)",
            "UPDATE owned_briefs SET target='/briefs/../Private.md'",
            "UPDATE occasions SET pack='Private invalid JSON'",
            "UPDATE occasions SET brief_path='/briefs/other.md'",
            "UPDATE owned_briefs SET write_offset=90000",
        ] {
            let before = ledger.brief(&row.id).unwrap();
            let result: Result<(), LedgerError> = ledger.transaction(|t| {
                t.execute(statement, []).map_err(map)?;
                let result = super::super::read::one(t, "occasion", row.id.as_str());
                assert!(matches!(result, Err(LedgerError::Corrupt(_))));
                assert!(!format!("{result:?}").contains("Private"));
                Err(LedgerError::Io("rollback".into()))
            });
            assert!(result.is_err());
            assert_eq!(ledger.brief(&row.id).unwrap(), before);
        }
        return true;
    }
    drop(ledger);
    let ledger = SqliteLedger::open(root).unwrap();
    assert_eq!(ledger.brief(&row.id).unwrap(), Some(artifact.clone()));
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    saved(publish(&ledger, run, &artifact, 11));
    let mut details = row.details.clone();
    details.at = OccasionTime::parse("2026-11-02T10:00:00Z").unwrap();
    details.title = "Private rescheduled title".into();
    let change = crate::ledger::occasion::prepare(
        &ledger,
        row.key.clone(),
        details,
        UtcInstant { secs: 12 },
    )
    .unwrap();
    saved(write(&ledger, run, change));
    drop(ledger);
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let mut expected = artifact.clone();
    expected.published_at = Some(UtcInstant { secs: 11 });
    assert_eq!(ledger.brief_at(&artifact.target).unwrap(), Some(expected));
    assert_eq!(
        ledger.occasion(&row.id).unwrap().unwrap().brief_path,
        Some(artifact.target)
    );
    true
}

fn migration(root: &crate::RootDir, ledger: SqliteLedger, row: &OccasionRecord) {
    ledger.read(|c| {
        crate::ledger::sqlite::tests::remove_release_extension(c)?;
        c.execute_batch("DROP TABLE owned_briefs; UPDATE occasions SET brief_path='/briefs/legacy.md',pack='Private legacy pack',briefed_at=3; PRAGMA user_version=12;").map_err(map)
    }).unwrap();
    drop(ledger);
    let before = crate::database::snapshot(root.path());
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert!(observer.brief(&row.id).unwrap().is_none());
    assert_eq!(observer.schema_version().unwrap(), 12);
    drop(observer);
    assert_eq!(crate::database::snapshot(root.path()), before);
    let mut db = crate::database::Database::open(
        root.try_clone().unwrap(),
        std::path::Path::new("vpt.db"),
        crate::database::AccessPolicy::WritableLedger,
        std::time::Duration::ZERO,
    )
    .unwrap();
    db.with_connection(|c| -> Result<(), LedgerError> {
        assert!(
            migrations::migrate(c, || Err(LedgerError::Io(
                "injected migration failure".into()
            )))
            .is_err()
        );
        assert_eq!(migrations::current(c)?, 12);
        let tables: i64 = c
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='owned_briefs'",
                [],
                |r| r.get(0),
            )
            .map_err(map)?;
        assert_eq!(tables, 0);
        Ok(())
    })
    .unwrap();
    drop(db);
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 18);
    assert!(upgraded.brief(&row.id).unwrap().is_none());
    upgraded
        .read(|c| {
            let pack: String = c
                .query_row("SELECT pack FROM occasions", [], |r| r.get(0))
                .map_err(map)?;
            assert_eq!(pack, "Private legacy pack");
            Ok(())
        })
        .unwrap();
    assert_eq!(
        upgraded.occasion(&row.id).unwrap().unwrap().briefed_at,
        Some(UtcInstant { secs: 3 })
    );
}
