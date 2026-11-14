use crate::ledger::contract::{
    self, briefs::fixture as brief, engine_outputs as engine, notes, releases,
};
use crate::{MemoryLedger, RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::time::UtcInstant;
mod atomic;
mod bounds;
mod execution;
mod expiration;
mod journal;
mod refusals;
mod regeneration;
mod sqlite;

fn inventory(
    ledger: &(
         impl RetentionArtifacts
         + RecordingLedger
         + RunHistory
         + PublicationJournal
         + OccasionLedger
         + ReleaseHistory
     ),
) {
    assert!(ledger.retention_artifacts().unwrap().is_empty());
    let run = notes::seed(ledger);
    engine::committed(ledger.commit_step(run, &engine::delta(), &engine::fact()));
    engine::committed(ledger.commit_step(run, &notes::delta(), &notes::facts()));
    let mut analysis = notes::delta();
    let note = &mut analysis.notes[0].artifact;
    note.stage = NoteStage::Analysis;
    note.target = "/analysis/pinned.md".into();
    note.text = note
        .text
        .replace("vpt-stage: transcript", "vpt-stage: analysis");
    note.digest = notes::hash(note.text.as_bytes());
    analysis.commit.publications[0].target = note.target.clone();
    analysis.commit.publications[0].intended = note.digest;
    engine::committed(ledger.commit_step(run, &analysis, &notes::facts()));
    let (brief_run, occasion) = brief::occasion(ledger);
    let brief = brief::artifact(&occasion, 10);
    engine::committed(brief::save(ledger, brief_run, &occasion, None, &brief));
    let (release_run, draft) = releases::setup(ledger);
    let release = engine::committed(ledger.prepare_release(release_run, &draft)).release;
    let rows = ledger.retention_artifacts().unwrap();
    assert_eq!(rows.len(), 8);
    assert!(rows.windows(2).all(|pair| pair[0].path < pair[1].path));
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == RetentionStatus::PublicationPending)
            .count(),
        6
    );
    assert!(rows.iter().any(|row| row.kind == ArtifactKind::Brief
        && row.owner == ArtifactOwner::Occasion(occasion.id.clone())
        && row.digest == brief.version.content));
    assert!(rows.iter().any(|row| row.kind == ArtifactKind::Release
        && row.path == release.destination
        && row.digest == release.version.content));
    assert!(
        rows.iter()
            .any(|row| row.kind == ArtifactKind::RedactionReport
                && row.path == release.report_path
                && row.digest == release.version.report)
    );
    for path in [
        &engine::artifact().target,
        &notes::artifact().target,
        &analysis.notes[0].artifact.target,
    ] {
        ledger.clear_publication(path).unwrap();
    }
    engine::committed(brief::publish(ledger, brief_run, &brief, 11));
    engine::committed(ledger.complete_release(
        release_run,
        release.sequence,
        release.version,
        draft.created_at,
    ));
    assert!(
        ledger
            .retention_artifacts()
            .unwrap()
            .iter()
            .all(|row| row.status == RetentionStatus::Published)
    );
    ledger
        .set_audio_trashed(&contract::record(81).id, UtcInstant { secs: 100 })
        .unwrap();
    let expired = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .filter(|row| row.status == RetentionStatus::Expired)
        .collect::<Vec<_>>();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].kind, ArtifactKind::Audio);
    assert_eq!(
        expired[0].owner,
        ArtifactOwner::Recording(contract::record(81).id)
    );
}

#[test]
fn memory_inventory_covers_every_owned_artifact_and_publication_state() {
    inventory(&MemoryLedger::new());
}

#[test]
fn sqlite_inventory_covers_every_owned_artifact_and_publication_state() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    inventory(&SqliteLedger::open(&root).unwrap());
}

fn wrong_kind(ledger: &(impl RecordingLedger + RunHistory + RetentionHistory + RetentionJournal)) {
    let record = contract::record(2);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record.clone()],
            ..Default::default()
        })
        .unwrap();
    let at = UtcInstant { secs: 200 };
    let run = engine::committed(ledger.begin_run(vpt_domain::run::RunKind::Retain, at)).run;
    let intent = NewIntent {
        kind: ArtifactKind::Transcript,
        owner: (record.id.clone()).into(),
        path: record.audio_path.clone(),
        expected: record.digest,
        recorded_at: at,
    };
    let prepared = engine::committed(ledger.prepare_retention(run, &intent));
    assert!(matches!(
        ledger.complete_retention(run, prepared.intent_id, prepared.effect_id, at),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_none()
    );
    assert_eq!(ledger.pending_intents().unwrap().len(), 1);
}

#[test]
fn memory_retention_kind_cannot_expire_another_artifact() {
    wrong_kind(&MemoryLedger::new());
}

#[test]
fn sqlite_retention_kind_cannot_expire_another_artifact() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    wrong_kind(&SqliteLedger::open(&root).unwrap());
}
