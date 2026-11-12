use super::super::SqliteLedger;
use crate::ledger::contract::releases::{committed, setup};
use vpt_application::ports::*;
#[test]
fn schema_fourteen_preserves_legacy_rows_and_rolls_back_incomplete_migration() {
    super::super::super::tests::supervised("ledger_releases_migration");
}
pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    let (run, mut draft) = setup(&ledger);
    let row = committed(ledger.prepare_release(run, &draft)).release;
    ledger.transaction(|c| c.execute_batch("DROP TABLE learned_tags; DROP TABLE tag_renames; DROP TABLE artifact_expirations; DROP TABLE release_payloads; DROP TABLE release_completions; DROP INDEX release_recording_sequence; DROP INDEX release_destination; DROP INDEX release_report_path; ALTER TABLE releases DROP COLUMN report_digest; PRAGMA user_version=13;").map_err(super::super::super::map)).unwrap();
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert!(observer.releases(&draft.recording).unwrap().is_empty());
    assert!(observer.pending_releases().unwrap().is_empty());
    assert_eq!(observer.schema_version().unwrap(), 13);
    let failure = ledger.read(|c| {
        super::super::super::migrations::migrate(c, || {
            Err(LedgerError::Io("migration interrupted".into()))
        })
    });
    assert!(failure.is_err());
    assert_eq!(ledger.schema_version().unwrap(), 13);
    ledger.read(|c| {
        let tables: i64 = c.query_row("SELECT count(*) FROM sqlite_schema WHERE name IN ('release_payloads','release_completions','tag_renames','learned_tags')", [], |r|r.get(0)).unwrap();
        assert_eq!(tables,0);
        Ok(())
    }).unwrap();
    let upgraded = SqliteLedger::open(root).unwrap();
    assert_eq!(upgraded.schema_version().unwrap(), 17);
    assert!(upgraded.releases(&draft.recording).unwrap().is_empty());
    upgraded
        .read(|c| {
            let digest: String = c
                .query_row(
                    "SELECT content_digest FROM releases WHERE sequence=?1",
                    [row.sequence],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(digest, row.version.content.hex());
            Ok(())
        })
        .unwrap();
    draft.destination = "/released/next.md".into();
    let next = committed(upgraded.prepare_release(run, &draft)).release;
    assert!(next.sequence > row.sequence);
}
