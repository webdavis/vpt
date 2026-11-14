use super::*;
use vpt_domain::run::RunKind;

pub(super) fn lifecycle(
    ledger: &(
         impl RetentionArtifacts
         + RecordingLedger
         + RunHistory
         + PublicationJournal
         + OccasionLedger
         + ReleaseHistory
         + RetentionHistory
         + RetentionJournal
         + TranscriptLedger
         + NoteLedger
         + BriefLedger
         + ReleaseLedger
     ),
) {
    inventory(ledger);
    let accepted = ledger
        .accepted_transcript(&contract::record(81).id)
        .unwrap();
    let transcript_note = ledger
        .note(&contract::record(81).id, NoteStage::Transcript)
        .unwrap();
    let analysis_note = ledger
        .note(&contract::record(81).id, NoteStage::Analysis)
        .unwrap();
    let occasion = ledger.occasions().unwrap().remove(0);
    let brief = ledger.brief(&occasion.id).unwrap();
    let release_sequence = ledger.releases(&contract::record(1).id).unwrap()[0];
    let release = ledger.release_payload(release_sequence).unwrap();
    let artifacts = ledger.retention_artifacts().unwrap();
    let at = UtcInstant { secs: 200 };
    let run = engine::committed(ledger.begin_run(RunKind::Retain, at)).run;
    for artifact in artifacts.iter().filter(|a| a.kind != ArtifactKind::Audio) {
        let expected = if matches!(
            artifact.kind,
            ArtifactKind::Transcript | ArtifactKind::Analysis | ArtifactKind::Brief
        ) {
            contract::digest(222)
        } else {
            artifact.digest
        };
        let intent = NewIntent {
            kind: artifact.kind,
            owner: artifact.owner.clone(),
            path: artifact.path.clone(),
            expected,
            recorded_at: at,
        };
        let prepared = engine::committed(ledger.prepare_retention(run, &intent));
        let receipt = engine::committed(ledger.complete_retention(
            run,
            prepared.intent_id,
            prepared.effect_id,
            at,
        ));
        assert_eq!(
            receipt.completed,
            artifact
                .owner
                .recording()
                .cloned()
                .into_iter()
                .collect::<Vec<_>>()
        );
        let expired = ledger
            .retention_artifacts()
            .unwrap()
            .into_iter()
            .find(|a| a.path == artifact.path)
            .unwrap();
        assert_eq!(expired.status, RetentionStatus::Expired);
        assert_eq!(expired.trashed_at, Some(at));
        assert_eq!(expired.digest, artifact.digest);
        assert_eq!(expired.owner, artifact.owner);
        let before = ledger.pending_exports().unwrap();
        let repeated = engine::committed(ledger.complete_retention(
            run,
            prepared.intent_id,
            prepared.effect_id,
            UtcInstant { secs: 300 },
        ));
        assert!(repeated.records.is_empty());
        assert_eq!(ledger.pending_exports().unwrap(), before);
        assert_eq!(
            ledger
                .retention_artifacts()
                .unwrap()
                .into_iter()
                .find(|a| a.path == artifact.path)
                .unwrap(),
            expired
        );
    }
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        ledger
            .accepted_transcript(&contract::record(81).id)
            .unwrap(),
        accepted
    );
    assert_eq!(
        ledger
            .note(&contract::record(81).id, NoteStage::Transcript)
            .unwrap(),
        transcript_note
    );
    assert_eq!(
        ledger
            .note(&contract::record(81).id, NoteStage::Analysis)
            .unwrap(),
        analysis_note
    );
    assert_eq!(ledger.brief(&occasion.id).unwrap(), brief);
    assert_eq!(ledger.release_payload(release_sequence).unwrap(), release);
    let record = ledger.by_id(&contract::record(81).id).unwrap().unwrap();
    assert_eq!(record.stages.note, StageState::Expired);
    assert_eq!(record.stages.analysis, StageState::Expired);
}

#[test]
fn memory_expiration_keeps_owned_metadata_and_original_times_for_every_later_store() {
    lifecycle(&MemoryLedger::new());
}

#[test]
fn sqlite_expiration_keeps_owned_metadata_and_original_times_after_reopen() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    lifecycle(&ledger);
    let before = ledger.retention_artifacts().unwrap();
    drop(ledger);
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(ledger.retention_artifacts().unwrap(), before);
}
