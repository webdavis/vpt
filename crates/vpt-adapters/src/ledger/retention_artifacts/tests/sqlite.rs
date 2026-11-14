use super::*;

#[test]
fn read_only_inventory_supports_an_older_schema_without_migrating_it() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    inventory(&ledger);
    ledger
        .transaction(|transaction| {
            transaction.pragma_update(None, "user_version", 1).unwrap();
            Ok(())
        })
        .unwrap();
    drop(ledger);
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    let before = std::fs::read(ledger.database_path()).unwrap();
    let rows = ledger.retention_artifacts().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.kind == ArtifactKind::Audio));
    assert_eq!(ledger.schema_version().unwrap(), 1);
    assert_eq!(std::fs::read(ledger.database_path()).unwrap(), before);
}

#[test]
fn pending_note_publication_takes_precedence_over_an_expired_stage() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    inventory(&ledger);
    ledger
        .transaction(|transaction| {
            transaction
                .execute_batch(
                    "UPDATE recordings SET stage_note='expired', stage_analysis='expired'",
                )
                .unwrap();
            Ok(())
        })
        .unwrap();
    for row in ledger
        .retention_artifacts()
        .unwrap()
        .iter()
        .filter(|row| matches!(row.kind, ArtifactKind::Transcript | ArtifactKind::Analysis))
    {
        assert_eq!(row.status, RetentionStatus::Expired);
    }
    ledger.commit(&notes::delta().commit).unwrap();
    let row = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|row| row.kind == ArtifactKind::Transcript)
        .unwrap();
    assert_eq!(row.status, RetentionStatus::PublicationPending);
}

#[test]
fn inventory_refuses_note_and_brief_paths_that_disagree_with_the_owner_pin() {
    for (table, column) in [
        ("recordings", "transcript_note_path"),
        ("recordings", "analysis_note_path"),
        ("occasions", "brief_path"),
    ] {
        let temp = crate::test_temp::tempdir().unwrap();
        let root = RootDir::open(temp.path()).unwrap();
        let ledger = SqliteLedger::open(&root).unwrap();
        inventory(&ledger);
        ledger
            .transaction(|transaction| {
                transaction
                    .execute_batch(&format!(
                        "UPDATE {table} SET {column}='/Private/foreign.md'"
                    ))
                    .unwrap();
                Ok(())
            })
            .unwrap();
        let error = ledger.retention_artifacts().unwrap_err();
        assert!(matches!(error, LedgerError::Corrupt(_)));
        assert!(!format!("{error:?}").contains("Private"));
    }
}

#[test]
fn metadata_inventory_does_not_parse_or_return_private_artifact_bodies() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    inventory(&ledger);
    let before = ledger.retention_artifacts().unwrap();
    ledger.transaction(|transaction| {
        transaction.execute_batch("UPDATE owned_notes SET body='Private broken note'; UPDATE owned_briefs SET body='Private broken brief'; UPDATE engine_outputs SET body=x'00FF'; UPDATE releases SET snapshot='Private broken provenance'").unwrap();
        Ok(())
    }).unwrap();
    assert_eq!(ledger.retention_artifacts().unwrap(), before);
    assert!(!format!("{before:?}").contains("Private"));
}

#[test]
fn malformed_sqlite_metadata_is_refused_before_allocating_owned_values() {
    for (column, value) in [
        ("audio_path", "a".repeat(65_537)),
        ("id", "Private malformed identity".into()),
        ("digest", "a".repeat(65)),
    ] {
        let temp = crate::test_temp::tempdir().unwrap();
        let root = RootDir::open(temp.path()).unwrap();
        let ledger = SqliteLedger::open(&root).unwrap();
        ledger
            .commit(&LedgerCommit {
                recordings: vec![contract::record(1)],
                ..Default::default()
            })
            .unwrap();
        ledger
            .transaction(|transaction| {
                transaction
                    .execute(&format!("UPDATE recordings SET {column}=?1"), [value])
                    .unwrap();
                Ok(())
            })
            .unwrap();
        let error = ledger.retention_artifacts().unwrap_err();
        assert!(matches!(error, LedgerError::Corrupt(_)));
        assert!(!format!("{error:?}").contains("Private"));
    }
}

#[test]
fn read_only_version_fourteen_inventory_and_journal_remain_observational() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    inventory(&ledger);
    let before = ledger.retention_artifacts().unwrap();
    let brief = before
        .iter()
        .find(|row| row.kind == ArtifactKind::Brief)
        .unwrap();
    let intent = NewIntent {
        kind: brief.kind,
        owner: brief.owner.clone(),
        path: brief.path.clone(),
        expected: brief.digest,
        recorded_at: UtcInstant { secs: 200 },
    };
    ledger.record_intent(&intent).unwrap();
    let pending = ledger.pending_intents().unwrap();
    ledger
        .transaction(|t| {
            t.execute_batch(
                "DROP TABLE action_sends; DROP TABLE learned_tags; DROP TABLE tag_renames; DROP TABLE artifact_expirations; PRAGMA user_version=14;",
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
    drop(ledger);
    let bytes = crate::database::snapshot(root.path());
    let observer = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(observer.retention_artifacts().unwrap(), before);
    assert_eq!(observer.pending_intents().unwrap(), pending);
    assert_eq!(observer.schema_version().unwrap(), 14);
    drop(observer);
    assert_eq!(crate::database::snapshot(root.path()), bytes);
    let migrated = SqliteLedger::open(&root).unwrap();
    assert_eq!(migrated.schema_version().unwrap(), 18);
    assert_eq!(migrated.retention_artifacts().unwrap(), before);
    assert_eq!(migrated.pending_intents().unwrap(), pending);
}
