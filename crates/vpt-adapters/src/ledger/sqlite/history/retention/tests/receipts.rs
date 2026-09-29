use super::*;
use crate::ledger::contract::retention_history::RetentionPairFixture;

pub(super) fn duplicate_times(ledger: &SqliteLedger) {
    let pair = ledger.seed_retention_pair();
    for (index, at) in [(0, 130), (1, 120), (0, 110), (1, 100)] {
        let receipt = committed(ledger.complete_retention(
            pair.run,
            pair.intents[index],
            pair.effects[index],
            UtcInstant { secs: at },
        ));
        if at <= 110 {
            assert!(receipt.records.is_empty());
        }
    }
    let times: (i64, i64, i64, u32) = ledger
        .read(|connection| {
            connection
                .query_row(
                    "SELECT (SELECT audio_trashed_at FROM recordings LIMIT 1), \
         (SELECT completed_at FROM retention_intents WHERE id = ?1), \
         (SELECT completed_at FROM retention_intents WHERE id = ?2), \
         (SELECT count(*) FROM history_records)",
                    params![pair.intents[0], pair.intents[1]],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .map_err(map)
        })
        .unwrap();
    assert_eq!(times, (130, 130, 120, 7));
}

pub(super) fn prepared_after_close(ledger: &SqliteLedger) {
    let run = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 119 })).run;
    let result = ledger.history_write_with(
        |transaction| prepare(transaction, run, &new_intent()),
        || {},
        |closed| {
            assert_eq!(closed, Ok(()));
            Err(LedgerError::Io("owned close failure".into()))
        },
    );
    let HistoryWrite::CommittedWithFailure { receipt, error } = result else {
        panic!("confirmed preparation receipt lost: {result:?}");
    };
    assert_eq!(error, LedgerError::Io("owned close failure".into()));
    assert!(receipt.intent_id > 0);
    assert_eq!(receipt.effect_id, receipt.receipt.records[0].id);
    assert_eq!(receipt.receipt.run, run);
    assert_eq!(ledger.pending_intents().unwrap()[0].id, receipt.intent_id);
    assert_eq!(
        ledger.recovery_view().unwrap().records.last(),
        receipt.receipt.records.last()
    );
}

pub(super) fn completed_after_root(ledger: &SqliteLedger, root: &RootDir) {
    let prepared = seeded(ledger);
    let held = root.path().with_file_name("held-state");
    let result = ledger.history_write_with(
        |transaction| {
            complete(
                transaction,
                prepared.receipt.run,
                prepared.intent_id,
                prepared.effect_id,
                UtcInstant { secs: 130 },
            )
        },
        || {
            std::fs::rename(root.path(), &held).unwrap();
            std::fs::create_dir(root.path()).unwrap();
        },
        |closed| closed,
    );
    let HistoryWrite::CommittedWithFailure { receipt, error } = result else {
        panic!("confirmed completion receipt lost: {result:?}");
    };
    assert!(matches!(error, LedgerError::PathEscape(_)), "{error:?}");
    assert_eq!(receipt.completed, vec![new_intent().recording]);
    assert_eq!(receipt.records.len(), 2);
    let retained = RootDir::open(&held).unwrap();
    let reopened = SqliteLedger::open(&retained).unwrap();
    assert_eq!(snapshot(&reopened), (1, Some(130), Some(130), 4));
    assert_eq!(
        &reopened.recovery_view().unwrap().records[2..],
        receipt.records.as_slice()
    );
}
