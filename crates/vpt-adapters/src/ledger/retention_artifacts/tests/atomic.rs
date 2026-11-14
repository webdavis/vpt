use super::*;
use vpt_domain::run::RunKind;

#[test]
fn sqlite_later_expiration_rolls_back_marker_stage_journal_and_history_together() {
    for clause in [
        "BEFORE UPDATE OF completed_at ON retention_intents",
        "BEFORE INSERT ON history_records WHEN json_extract(NEW.fact, '$.tag')='work'",
    ] {
        let temp = crate::test_temp::tempdir().unwrap();
        let root = RootDir::open(temp.path()).unwrap();
        let ledger = SqliteLedger::open(&root).unwrap();
        inventory(&ledger);
        let at = UtcInstant { secs: 200 };
        let run = engine::committed(ledger.begin_run(RunKind::Retain, at)).run;
        let artifact = ledger
            .retention_artifacts()
            .unwrap()
            .into_iter()
            .find(|a| a.kind == ArtifactKind::Transcript)
            .unwrap();
        let intent = NewIntent {
            kind: artifact.kind,
            owner: artifact.owner,
            path: artifact.path,
            expected: artifact.digest,
            recorded_at: at,
        };
        let prepared = engine::committed(ledger.prepare_retention(run, &intent));
        let before = (
            ledger.retention_artifacts().unwrap(),
            ledger.pending_exports().unwrap(),
            ledger.pending_intents().unwrap(),
            ledger.by_id(&contract::record(81).id).unwrap(),
        );
        ledger.transaction(|t| {
            t.execute_batch(&format!("CREATE TRIGGER fail_expiration {clause} BEGIN SELECT RAISE(ABORT, 'injected fault'); END;")).unwrap(); Ok(())
        }).unwrap();
        assert!(
            matches!(
                ledger.complete_retention(run, prepared.intent_id, prepared.effect_id, at),
                HistoryWrite::Unconfirmed(_)
            ),
            "{clause}"
        );
        assert_eq!(
            (
                ledger.retention_artifacts().unwrap(),
                ledger.pending_exports().unwrap(),
                ledger.pending_intents().unwrap(),
                ledger.by_id(&contract::record(81).id).unwrap()
            ),
            before
        );
        ledger
            .transaction(|t| {
                t.execute_batch("DROP TRIGGER fail_expiration").unwrap();
                Ok(())
            })
            .unwrap();
        engine::committed(ledger.complete_retention(
            run,
            prepared.intent_id,
            prepared.effect_id,
            at,
        ));
        let duplicate = engine::committed(ledger.prepare_retention(run, &intent));
        engine::committed(ledger.complete_retention(
            run,
            duplicate.intent_id,
            duplicate.effect_id,
            UtcInstant { secs: 300 },
        ));
        let row = ledger
            .retention_artifacts()
            .unwrap()
            .into_iter()
            .find(|a| a.kind == ArtifactKind::Transcript)
            .unwrap();
        assert_eq!(row.trashed_at, Some(at));
    }
}

#[test]
fn sqlite_inventory_refuses_expiration_that_no_longer_matches_owned_metadata() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    expiration::lifecycle(&ledger);
    ledger
        .transaction(|t| {
            t.execute(
                "UPDATE artifact_expirations SET digest=?1 WHERE kind='transcript'",
                [contract::digest(99).hex()],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        ledger.retention_artifacts(),
        Err(LedgerError::Corrupt(_))
    ));
}
