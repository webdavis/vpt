use super::super::SqliteLedger;
use crate::ledger::contract::releases::{committed, setup};
use vpt_application::ports::*;
use vpt_domain::time::UtcInstant;
#[test]
fn release_preparation_and_completion_roll_back_with_failed_payload_or_history() {
    super::super::super::tests::supervised("ledger_releases_fault_rollback");
}
#[test]
fn reopened_releases_keep_exact_pending_bytes_and_frozen_finalized_metadata() {
    super::super::super::tests::supervised("ledger_releases_fault_reopen");
}
#[test]
fn changed_payload_and_malformed_snapshot_refuse_without_private_diagnostics() {
    super::super::super::tests::supervised("ledger_releases_fault_corrupt");
}
fn sql(ledger: &SqliteLedger, text: &str) {
    ledger
        .transaction(|c| c.execute_batch(text).map_err(super::super::super::map))
        .unwrap();
}
pub(super) fn run(scenario: &str, root: &crate::RootDir) -> bool {
    if !scenario.starts_with("ledger_releases_fault_") {
        return false;
    }
    let ledger = SqliteLedger::open(root).unwrap();
    let (run, draft) = setup(&ledger);
    match scenario {
        "ledger_releases_fault_rollback" => {
            let before = ledger.pending_exports().unwrap();
            sql(
                &ledger,
                "CREATE TRIGGER release_payload_failure BEFORE INSERT ON release_payloads BEGIN SELECT RAISE(ABORT,'private'); END",
            );
            assert!(matches!(
                ledger.prepare_release(run, &draft),
                HistoryWrite::Unconfirmed(_)
            ));
            assert_eq!(ledger.pending_exports().unwrap(), before);
            assert!(ledger.release(1).unwrap().is_none());
            sql(&ledger, "DROP TRIGGER release_payload_failure");
            let prepared = committed(ledger.prepare_release(run, &draft));
            assert_eq!(prepared.release.sequence, 1);
            let before = ledger.pending_exports().unwrap();
            sql(
                &ledger,
                "CREATE TRIGGER release_history_failure BEFORE INSERT ON history_records WHEN json_extract(NEW.fact,'$.tag')='work' BEGIN SELECT RAISE(ABORT,'private'); END",
            );
            assert!(matches!(
                ledger.complete_release(run, 1, prepared.release.version, draft.created_at),
                HistoryWrite::Unconfirmed(_)
            ));
            assert_eq!(ledger.pending_exports().unwrap(), before);
            assert!(ledger.releases(&draft.recording).unwrap().is_empty());
            assert!(ledger.release_payload(1).unwrap().is_some());
            sql(&ledger, "DROP TRIGGER release_history_failure");
            committed(ledger.complete_release(run, 1, prepared.release.version, draft.created_at));
        }
        "ledger_releases_fault_reopen" => {
            let row = committed(ledger.prepare_release(run, &draft)).release;
            drop(ledger);
            let ledger = SqliteLedger::open(root).unwrap();
            let payload = ledger.release_payload(row.sequence).unwrap().unwrap();
            assert_eq!(payload.text, draft.text);
            assert_eq!(payload.report, draft.report);
            assert_eq!(payload.release, row);
            committed(ledger.complete_release(
                run,
                row.sequence,
                row.version,
                UtcInstant {
                    secs: 1_800_000_002,
                },
            ));
            drop(ledger);
            let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
            assert_eq!(ledger.releases(&draft.recording).unwrap(), [row.sequence]);
            assert!(ledger.release_payload(row.sequence).unwrap().is_none());
            assert_eq!(
                ledger.release(row.sequence).unwrap().unwrap().provenance,
                row.provenance
            );
        }
        "ledger_releases_fault_corrupt" => {
            let row = committed(ledger.prepare_release(run, &draft)).release;
            sql(
                &ledger,
                "UPDATE release_payloads SET report='Private changed report'",
            );
            let error = ledger.release_payload(row.sequence).unwrap_err();
            assert!(!format!("{error:?}").contains("Private"));
            assert!(matches!(
                ledger.complete_release(run, row.sequence, row.version, draft.created_at),
                HistoryWrite::Unconfirmed(_)
            ));
            sql(
                &ledger,
                "UPDATE releases SET snapshot='Private invalid snapshot'",
            );
            let error = ledger.release(row.sequence).unwrap_err();
            assert!(!format!("{error:?}").contains("Private"));
        }
        _ => panic!("unknown release fault"),
    }
    true
}
