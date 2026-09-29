use super::{digest, ingest, publication, record, record_at, seen_row};
use std::path::{Path, PathBuf};
use vpt_application::ports::{
    LedgerCommit, LedgerError, PublicationJournal, RecordingLedger, StageState, TitleOrigin,
};
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::UtcInstant;

pub(crate) fn seen_is_absent_until_recorded_and_updated<L: RecordingLedger>(ledger: &L) {
    let mut row = seen_row("/vm/Recordings/a.m4a");
    assert_eq!(ledger.seen(&row.path).unwrap(), None);
    row.flags = 0x8000;
    ledger.record_seen(&row).unwrap();
    assert_eq!(ledger.seen(&row.path).unwrap(), Some(row.clone()));
    row.deferral_count = 1;
    row.deferral_reason = Some(DeferralReason::NotAtRest);
    row.deferred_size = Some(1_024);
    ledger.record_seen(&row).unwrap();
    assert_eq!(ledger.seen(&row.path).unwrap(), Some(row));
    assert_eq!(ledger.seen_all().unwrap().len(), 1);
}

pub(crate) fn second_record_preserves_first_seen<L: RecordingLedger>(ledger: &L) {
    let mut row = seen_row("/vm/Recordings/a.m4a");
    ledger.record_seen(&row).unwrap();
    row.first_seen = UtcInstant {
        secs: 1_787_800_000,
    };
    row.last_seen = row.first_seen;
    ledger.record_seen(&row).unwrap();
    let stored = ledger.seen(&row.path).unwrap().unwrap();
    assert_eq!(
        stored.first_seen,
        UtcInstant {
            secs: 1_787_700_000
        }
    );
    assert_eq!(stored.last_seen, row.last_seen);
}

pub(crate) fn seen_all_is_ordered_by_path<L: RecordingLedger>(ledger: &L) {
    let seen = ["c.m4a", "a/child.m4a", "a.m4a", "b.m4a"]
        .into_iter()
        .map(|name| seen_row(&format!("/vm/Recordings/{name}")))
        .collect();
    ledger
        .commit(&LedgerCommit {
            seen,
            ..Default::default()
        })
        .unwrap();
    let paths: Vec<_> = ledger
        .seen_all()
        .unwrap()
        .into_iter()
        .map(|row| row.path)
        .collect();
    assert_eq!(
        paths,
        ["a.m4a", "a/child.m4a", "b.m4a", "c.m4a"]
            .map(|name| PathBuf::from(format!("/vm/Recordings/{name}")))
    );
}

pub(crate) fn commit_writes_recording_seen_and_publication<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let recording = record(1);
    let mut seen = seen_row("/vm/Recordings/1.m4a");
    seen.recording = Some(recording.id.clone());
    let entry = publication("/h/transcripts/1.md", 11, 1);
    let mut batch = ingest(recording.clone(), seen.clone());
    batch.publications.push(entry.clone());
    ledger.commit(&batch).unwrap();
    assert_eq!(
        ledger.by_id(&recording.id).unwrap(),
        Some(recording.clone())
    );
    assert_eq!(
        ledger.by_digest(&recording.digest).unwrap(),
        Some(recording.clone())
    );
    assert_eq!(ledger.seen(&seen.path).unwrap(), Some(seen));
    assert_eq!(ledger.recordings().unwrap(), vec![recording]);
    assert_eq!(ledger.pending_publications().unwrap(), vec![entry]);
}

pub(crate) fn stored_conflict_rolls_back_earlier_rows<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let first = record(2);
    ledger
        .commit(&ingest(first.clone(), seen_row("/vm/Recordings/2.m4a")))
        .unwrap();
    let conflicting = record_at(
        2,
        UtcInstant {
            secs: first.captured_at.secs + 1,
        },
    );
    let path = Path::new("/vm/Recordings/2b.m4a");
    let mut batch = ingest(conflicting, seen_row(path.to_str().unwrap()));
    batch
        .publications
        .push(publication("/h/transcripts/2b.md", 12, 1));
    assert!(matches!(
        ledger.commit(&batch),
        Err(LedgerError::Conflict(_))
    ));
    assert_eq!(ledger.seen(path).unwrap(), None);
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(ledger.recordings().unwrap(), vec![first]);
}

pub(crate) fn within_batch_conflict_rolls_back_earlier_rows<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let first = record(3);
    let conflicting = record_at(
        3,
        UtcInstant {
            secs: first.captured_at.secs + 1,
        },
    );
    let path = Path::new("/vm/Recordings/3.m4a");
    let batch = LedgerCommit {
        recordings: vec![first, conflicting],
        seen: vec![seen_row(path.to_str().unwrap())],
        publications: vec![publication("/h/transcripts/3.md", 13, 1)],
    };
    assert!(matches!(
        ledger.commit(&batch),
        Err(LedgerError::Conflict(_))
    ));
    assert_eq!(ledger.seen(path).unwrap(), None);
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(ledger.recordings().unwrap().is_empty());
}

pub(crate) fn source_path_update_preserves_other_recording_fields<L: RecordingLedger>(ledger: &L) {
    let mut recording = record(4);
    recording.audio_trashed_at = Some(UtcInstant {
        secs: 1_787_800_000,
    });
    ledger
        .commit(&ingest(recording.clone(), seen_row("/vm/Recordings/4.m4a")))
        .unwrap();
    let path = Path::new("/vm/Recordings/renamed.m4a");
    ledger.set_source_path(&recording.id, path).unwrap();
    recording.source_path = Some(path.to_path_buf());
    assert_eq!(ledger.by_id(&recording.id).unwrap(), Some(recording));
}

pub(crate) fn audio_trashed_update_preserves_other_recording_fields<L: RecordingLedger>(
    ledger: &L,
) {
    let mut recording = record(4);
    recording.source_path = Some(PathBuf::from("/vm/Recordings/renamed.m4a"));
    ledger
        .commit(&ingest(recording.clone(), seen_row("/vm/Recordings/4.m4a")))
        .unwrap();
    let at = UtcInstant {
        secs: 1_787_800_000,
    };
    ledger.set_audio_trashed(&recording.id, at).unwrap();
    recording.audio_trashed_at = Some(at);
    assert_eq!(ledger.by_id(&recording.id).unwrap(), Some(recording));
}

pub(crate) fn unknown_identity_reads_as_absent<L: RecordingLedger>(ledger: &L) {
    let identity = record(5).id;
    assert_eq!(ledger.by_id(&identity).unwrap(), None);
    assert_eq!(ledger.by_digest(&digest(5)).unwrap(), None);
}

pub(crate) fn recording_without_source_or_seen_commits<L: RecordingLedger>(ledger: &L) {
    let mut recording = record(6);
    recording.source_path = None;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ledger.by_id(&recording.id).unwrap(), Some(recording));
    assert!(ledger.seen_all().unwrap().is_empty());
}

pub(crate) fn recordings_order_by_capture_then_identity<L: RecordingLedger>(ledger: &L) {
    let at = UtcInstant {
        secs: 1_787_604_456,
    };
    let later = record_at(8, UtcInstant { secs: at.secs + 1 });
    let higher = record_at(7, at);
    let lower = record_at(6, at);
    assert!(lower.id.as_str() < higher.id.as_str());
    for recording in [later.clone(), higher.clone(), lower.clone()] {
        ledger
            .commit(&LedgerCommit {
                recordings: vec![recording],
                ..Default::default()
            })
            .unwrap();
    }
    assert_eq!(ledger.recordings().unwrap(), vec![lower, higher, later]);
}

pub(crate) fn different_stage_values_round_trip<L: RecordingLedger>(ledger: &L) {
    let mut recording = record(9);
    recording.title_source = TitleOrigin::Unavailable;
    recording.stages.transcribe = StageState::Succeeded;
    recording.stages.note = StageState::Failed;
    recording.stages.synthesis = StageState::Disabled;
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ledger.by_id(&recording.id).unwrap(), Some(recording));
}

pub(crate) fn repeated_seen_and_clear_are_idempotent<L>(ledger: &L)
where
    L: RecordingLedger + PublicationJournal,
{
    let row = seen_row("/vm/Recordings/repeated.m4a");
    ledger.record_seen(&row).unwrap();
    ledger.record_seen(&row).unwrap();
    ledger
        .clear_publication(Path::new("/h/transcripts/missing.md"))
        .unwrap();
    ledger
        .clear_publication(Path::new("/h/transcripts/missing.md"))
        .unwrap();
    assert_eq!(ledger.seen_all().unwrap(), vec![row]);
    assert!(ledger.pending_publications().unwrap().is_empty());
}

pub(crate) fn commit_seen_replacement_preserves_first_seen<L: RecordingLedger>(ledger: &L) {
    let mut row = seen_row("/vm/Recordings/replacement.m4a");
    ledger
        .commit(&LedgerCommit {
            seen: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    row.first_seen.secs += 100;
    row.last_seen.secs += 100;
    ledger
        .commit(&LedgerCommit {
            seen: vec![row.clone()],
            ..Default::default()
        })
        .unwrap();
    let stored = ledger.seen(&row.path).unwrap().unwrap();
    assert_eq!(stored.first_seen.secs, row.first_seen.secs - 100);
    assert_eq!(stored.last_seen, row.last_seen);
}
