use super::super::{SqliteLedger, map, migrations};
use crate::ledger::contract::occasions::{details, saved, write};
use vpt_application::ports::*;
use vpt_domain::{
    occasion::{OccasionKey, time::OccasionTime},
    run::RunKind,
    time::UtcInstant,
};

#[test]
fn rescheduled_occasion_preserves_first_pin_pack_and_briefed_marker_after_reopen() {
    super::super::tests::supervised("ledger_occasions_durability");
}
#[test]
fn version_eleven_occasion_upgrade_preserves_legacy_rows_without_inventing_offset() {
    super::super::tests::supervised("ledger_occasions_migration");
}

pub(super) fn run(scenario: &str, root: &crate::RootDir) -> bool {
    if !matches!(
        scenario,
        "ledger_occasions_durability" | "ledger_occasions_migration"
    ) {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let at = UtcInstant { secs: 2 };
    let run = saved(ledger.begin_run(RunKind::Brief, at)).run;
    let key = OccasionKey::google("Private calendar", "Private occurrence").unwrap();
    let first = crate::ledger::occasion::prepare(&ledger, key.clone(), details(), at).unwrap();
    let id = first.occasion.id.clone();
    saved(write(&ledger, run, first));
    ledger.transaction(|t| t.execute("UPDATE occasions SET brief_path='/briefs/original.md', pack='Private saved pack', briefed_at=3", []).map(|_| ()).map_err(map)).unwrap();
    let before = ledger.occasion(&id).unwrap().unwrap();
    let history = ledger.pending_exports().unwrap();
    if scenario == "ledger_occasions_migration" {
        ledger
            .read(|c| {
                crate::ledger::sqlite::tests::remove_release_extension(c)?;
                c.execute_batch(
                    "DROP TABLE owned_briefs; ALTER TABLE occasions DROP COLUMN at_rfc3339; PRAGMA user_version=11;",
                )
                .map_err(map)
            })
            .unwrap();
        drop(ledger);
        let bytes = crate::database::snapshot(root.path());
        let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
        assert_eq!(observer.schema_version().unwrap(), 11);
        assert!(matches!(observer.occasions(), Err(LedgerError::Corrupt(_))));
        assert_eq!(observer.pending_exports().unwrap(), history);
        drop(observer);
        assert_eq!(crate::database::snapshot(root.path()), bytes);
        let mut db = crate::database::Database::open(
            root.try_clone().unwrap(),
            std::path::Path::new("vpt.db"),
            crate::database::AccessPolicy::WritableLedger,
            std::time::Duration::ZERO,
        )
        .unwrap();
        db.with_connection(|c| {
            let error = LedgerError::Io("injected migration failure".into());
            assert_eq!(migrations::migrate(c, || Err(error.clone())), Err(error));
            assert_eq!(migrations::current(c)?, 11);
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM pragma_table_info('occasions') WHERE name='at_rfc3339'",
                    [],
                    |row| row.get::<_, u32>(0)
                )
                .unwrap(),
                0
            );
            Ok::<_, LedgerError>(())
        })
        .unwrap();
        db.try_close().unwrap();
        let upgraded = SqliteLedger::open(root).unwrap();
        assert_eq!(upgraded.schema_version().unwrap(), 19);
        assert!(matches!(upgraded.occasions(), Err(LedgerError::Corrupt(_))));
        upgraded
            .read(|c| {
                let value: (String, String, Option<String>) = c
                    .query_row(
                        "SELECT brief_path, pack, at_rfc3339 FROM occasions",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .unwrap();
                assert_eq!(
                    value,
                    (
                        "/briefs/original.md".into(),
                        "Private saved pack".into(),
                        None
                    )
                );
                Ok(())
            })
            .unwrap();
        return true;
    }
    drop(ledger);
    let ledger = SqliteLedger::open(root).unwrap();
    assert_eq!(ledger.occasion(&id).unwrap(), Some(before.clone()));
    let mut next = details();
    next.title = "Private rescheduled title".into();
    next.at = OccasionTime::parse("2026-10-08T10:00:00+02:00").unwrap();
    let change =
        crate::ledger::occasion::prepare(&ledger, key, next, UtcInstant { secs: 4 }).unwrap();
    assert_eq!(change.occasion.id, id);
    assert_eq!(change.occasion.brief_path, before.brief_path);
    saved(write(&ledger, run, change));
    ledger
        .read(|c| {
            assert_eq!(
                c.query_row("SELECT pack FROM occasions", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "Private saved pack"
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(
        ledger.occasion(&id).unwrap().unwrap().briefed_at,
        Some(UtcInstant { secs: 3 })
    );
    true
}
