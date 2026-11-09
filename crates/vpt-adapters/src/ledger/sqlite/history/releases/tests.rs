use super::*;
use crate::ledger::contract::releases::{committed, setup};
use vpt_application::ports::ReleaseLedger;
#[test]
fn close_failure_preserves_committed_release_receipts_and_recovery_payloads() {
    super::super::super::tests::supervised("ledger_releases_ack_failure");
}
pub(in crate::ledger::sqlite) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    let (run, draft) = setup(&ledger);
    let result = ledger.history_write_with(
        |c| prepare::apply(c, run, &draft),
        || {},
        |_| Err(LedgerError::Io("close acknowledgement".into())),
    );
    let HistoryWrite::CommittedWithFailure {
        receipt: prepared, ..
    } = result
    else {
        panic!("lost committed release");
    };
    assert_eq!(
        ledger.pending_releases().unwrap(),
        [prepared.release.sequence]
    );
    assert_eq!(
        ledger
            .release_payload(prepared.release.sequence)
            .unwrap()
            .unwrap()
            .report,
        draft.report
    );
    let result = ledger.history_write_with(
        |c| {
            complete::apply(
                c,
                run,
                prepared.release.sequence,
                prepared.release.version,
                draft.created_at,
            )
        },
        || {},
        |_| Err(LedgerError::Io("close acknowledgement".into())),
    );
    let HistoryWrite::CommittedWithFailure { receipt, .. } = result else {
        panic!("lost completed release");
    };
    assert_eq!(receipt.completed, std::slice::from_ref(&draft.recording));
    assert_eq!(
        ledger.releases(&draft.recording).unwrap(),
        [prepared.release.sequence]
    );
    let repeat = committed(ledger.complete_release(
        run,
        prepared.release.sequence,
        prepared.release.version,
        draft.created_at,
    ));
    assert!(repeat.records.is_empty());
}
