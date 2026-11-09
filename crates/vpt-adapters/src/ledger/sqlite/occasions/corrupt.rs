use super::super::{SqliteLedger, map};
use crate::ledger::contract::occasions::{details, saved, write};
use vpt_application::ports::*;
use vpt_domain::{occasion::OccasionKey, run::RunKind, time::UtcInstant};

#[test]
fn malformed_occasion_rows_fail_privately_without_changing_saved_bytes() {
    super::super::tests::supervised("ledger_occasions_corrupt");
}

pub(super) fn run(root: &crate::RootDir) {
    let ledger = SqliteLedger::open(root).unwrap();
    let at = UtcInstant { secs: 2 };
    let run = saved(ledger.begin_run(RunKind::Brief, at)).run;
    let row = crate::ledger::occasion::prepare(
        &ledger,
        OccasionKey::dam("Private id").unwrap(),
        details(),
        at,
    )
    .unwrap();
    let id = row.occasion.id.clone();
    saved(write(&ledger, run, row));
    let history = ledger.pending_exports().unwrap();
    for sql in [
        "UPDATE occasions SET brief_path='/briefs/./private.md'",
        "UPDATE occasions SET at_rfc3339=NULL",
        "UPDATE occasions SET at_rfc3339='Private malformed timestamp'",
        "UPDATE occasions SET provider_key='dam:007:Private'",
        "UPDATE occasions SET source='google'",
        "UPDATE occasions SET participants='[{}]'",
        "UPDATE occasions SET duration_secs=-1",
        "UPDATE occasions SET brief_path=zeroblob(65537)",
        "UPDATE occasions SET briefed_at=1",
        "UPDATE occasions SET at=at+1",
        "UPDATE occasions SET brief_path='/briefs/private.md', briefed_at=9223372036854775807",
    ] {
        let injected = LedgerError::Io("rollback corrupt fixture".into());
        let result = ledger.transaction(|c| {
            c.execute(sql, []).map_err(map)?;
            let error = super::read::one(c, "id = ?1", id.as_str()).unwrap_err();
            assert!(matches!(error, LedgerError::Corrupt(_)), "{sql}");
            assert!(!format!("{error:?}").contains("Private"));
            Err::<(), _>(injected.clone())
        });
        assert_eq!(result, Err(injected));
    }
    assert_eq!(ledger.pending_exports().unwrap(), history);
    assert_eq!(ledger.occasion(&id).unwrap().unwrap().details, details());
}
