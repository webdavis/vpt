use super::*;
use vpt_domain::run::RunKind;

fn regenerate(
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
    expiration::lifecycle(ledger);
    let at = UtcInstant { secs: 400 };
    let run = engine::committed(ledger.begin_run(RunKind::Publish, at)).run;
    let mut delta = notes::delta();
    delta.notes[0].expected_saved = Some(notes::artifact().digest);
    delta.commit.publications[0].expected_previous = Some(notes::artifact().digest);
    engine::committed(ledger.commit_step(run, &delta, &notes::facts()));
    let note = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == ArtifactKind::Transcript)
        .unwrap();
    assert_eq!(note.status, RetentionStatus::PublicationPending);
    assert_eq!(note.trashed_at, None);
    engine::committed(ledger.commit_step(
        run,
        &WorkDelta {
            publication_clears: vec![note.path.clone()],
            ..Default::default()
        },
        &notes::facts(),
    ));
    let restored = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|a| a.path == note.path)
        .unwrap();
    assert_eq!(restored.status, RetentionStatus::Published);
    let occasion = ledger.occasions().unwrap().remove(0);
    let previous = ledger.brief(&occasion.id).unwrap().unwrap();
    let mut regenerated = brief::artifact(&occasion, 400);
    regenerated.first_written = previous.first_written;
    engine::committed(brief::save(
        ledger,
        run,
        &occasion,
        Some(previous.version),
        &regenerated,
    ));
    engine::committed(brief::publish(ledger, run, &regenerated, 401));
    let artifacts = ledger.retention_artifacts().unwrap();
    let restored = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Brief)
        .unwrap();
    assert_eq!(restored.status, RetentionStatus::Published);
    assert_eq!(restored.trashed_at, None);
    assert_eq!(
        artifacts
            .iter()
            .filter(|a| a.trashed_at == Some(UtcInstant { secs: 200 }))
            .count(),
        4
    );
}

fn pending_engine(
    ledger: &(
         impl RetentionArtifacts
         + RecordingLedger
         + RunHistory
         + PublicationJournal
         + OccasionLedger
         + ReleaseHistory
         + RetentionHistory
         + RetentionJournal
     ),
) {
    inventory(ledger);
    let at = UtcInstant { secs: 200 };
    let run = engine::committed(ledger.begin_run(RunKind::Retain, at)).run;
    let output = engine::artifact();
    engine::committed(ledger.prepare_retention(
        run,
        &NewIntent {
            kind: ArtifactKind::EngineOutput,
            owner: output.recording.clone().into(),
            path: output.target.clone(),
            expected: output.digest,
            recorded_at: at,
        },
    ));
    let before = ledger.pending_exports().unwrap();
    let mut delta = engine::delta();
    delta.commit.publications[0].expected_previous = Some(output.digest);
    assert!(matches!(
        ledger.commit_step(run, &delta, &engine::fact()),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_exports().unwrap(), before);
    assert!(ledger.pending_publications().unwrap().is_empty());
}

#[test]
fn memory_deliberate_regeneration_clears_only_its_expiration() {
    regenerate(&MemoryLedger::new());
}
#[test]
fn sqlite_deliberate_regeneration_clears_only_its_expiration() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    regenerate(&SqliteLedger::open(&root).unwrap());
}
#[test]
fn memory_pending_retention_refuses_engine_republication() {
    pending_engine(&MemoryLedger::new());
}
#[test]
fn sqlite_pending_retention_refuses_engine_republication() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    pending_engine(&SqliteLedger::open(&root).unwrap());
}
