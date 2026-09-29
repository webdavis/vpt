use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use vpt_application::ports::{
    LedgerCommit, RecorderStore, RecordingRecord, SeenRow, StageStates, TitleOrigin,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

fn first_take_snapshot(parts: &Parts, source: &Path) -> LedgerCommit {
    let candidate = parts.store.candidate(source).unwrap();
    let digest = Sha256Digest(Sha256::digest(std::fs::read(source).unwrap()).into());
    let captured_at = UtcInstant { secs: CAPTURED };
    let id = RecordingId::derive(captured_at, parts.clock.offset, &digest).unwrap();
    LedgerCommit {
        recordings: vec![RecordingRecord {
            audio_path: parts.archive.target(&format!("{id}.m4a")),
            id: id.clone(),
            source_path: Some(source.to_path_buf()),
            digest,
            captured_at,
            captured_offset: parts.clock.offset,
            duration_secs: 1,
            title: None,
            title_source: TitleOrigin::Unavailable,
            ingested_at: parts.clock.now,
            stages: StageStates::fresh(),
            audio_trashed_at: None,
        }],
        seen: vec![SeenRow {
            path: candidate.path,
            file_name: candidate.file_name,
            size: candidate.size,
            mtime: candidate.mtime,
            flags: candidate.flags,
            first_seen: parts.clock.now,
            last_seen: parts.clock.now,
            deferral_count: 0,
            deferral_reason: None,
            deferred_size: None,
            source_gone_at: None,
            recording: Some(id),
        }],
        publications: Vec::new(),
    }
}

#[test]
fn first_take_snapshot_matches_completed_ingest() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 1, b"first take");
    let source = fixture.add_recording("edited.m4a", &bytes);
    let mut parts = fixture.parts();
    let expected = first_take_snapshot(&parts, &source);

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.ingested, expected.recordings);
    assert_eq!(parts.ledger.recordings().unwrap(), expected.recordings);
    assert_eq!(parts.ledger.seen_all().unwrap(), expected.seen);
    let archived = &expected.recordings[0].audio_path;
    assert_eq!(std::fs::read(archived).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(archived).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
}

fn partial_source_snapshot(parts: &Parts, source: &Path) -> SeenRow {
    let candidate = parts.store.candidate(source).unwrap();
    SeenRow {
        path: candidate.path,
        file_name: candidate.file_name,
        size: candidate.size,
        mtime: candidate.mtime,
        flags: candidate.flags,
        first_seen: parts.clock.now,
        last_seen: parts.clock.now,
        deferral_count: 1,
        deferral_reason: Some(DeferralReason::InvalidContainer),
        deferred_size: Some(candidate.size),
        source_gone_at: None,
        recording: None,
    }
}

#[test]
fn partial_source_snapshot_matches_committed_deferral() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"owned payload");
    let source = fixture.add_recording("growing.m4a", &whole[..whole.len() - 12]);
    let mut parts = fixture.parts();
    let expected = partial_source_snapshot(&parts, &source);

    let first = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(first.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(parts.ledger.seen(&source).unwrap(), Some(expected));
}

#[test]
fn dataless_defers_before_open_and_preserves_observed_flags() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("cloud.m4a", &m4a(CAPTURED, 1, b"owned audio"));
    let mut parts = fixture.parts();
    let probe = GateRecorder::new(&fixture.recordings);
    probe.extra_flags.set(SF_DATALESS | 0x20);

    let report = probed(&mut parts, &probe).expect("dataless deferral");

    assert_eq!(
        report.deferred,
        vec![vpt_application::Deferred {
            path: source.clone(),
            reason: DeferralReason::Dataless
        }]
    );
    assert_eq!(probe.opens.get(), 0);
    assert_eq!(probe.reads.get(), 0);
    assert_eq!(probe.clones.get(), 0);
    let seen = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(
        (seen.deferral_count, seen.deferral_reason, seen.flags),
        (1, Some(DeferralReason::Dataless), SF_DATALESS | 0x20)
    );
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.ledger.recordings().unwrap().is_empty());
}

#[test]
fn oversized_descriptor_defers_before_read_or_clone() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("big.m4a", &m4a(CAPTURED, 1, &[0u8; 4096]));
    let mut parts = fixture.parts();
    parts.settings.max_audio_bytes = std::fs::metadata(&source).unwrap().len() - 1;
    let probe = GateRecorder::new(&fixture.recordings);

    let report = probed(&mut parts, &probe).expect("size deferral");

    assert_eq!(report.deferred[0].reason, DeferralReason::AudioTooLarge);
    assert_eq!(probe.reads.get(), 0);
    assert_eq!(probe.clones.get(), 0);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
}

#[test]
fn exact_size_limit_is_eligible() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("exact.m4a", &m4a(CAPTURED, 1, b"owned audio"));
    let mut parts = fixture.parts();
    parts.settings.max_audio_bytes = std::fs::metadata(&source).unwrap().len();
    let probe = GateRecorder::new(&fixture.recordings);

    let report = probed(&mut parts, &probe).expect("equal size accepted");

    assert_eq!(report.ingested.len(), 1);
    assert!(report.deferred.is_empty());
    assert!(probe.reads.get() > 0);
}

#[test]
fn invalid_source_container_defers_before_staging() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("partial.m4a", b"incomplete owned container");
    let mut parts = fixture.parts();
    let probe = GateRecorder::new(&fixture.recordings);

    let report = probed(&mut parts, &probe).expect("invalid source deferral");

    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(probe.clones.get(), 0);
    assert!(parts.ledger.seen(&source).unwrap().is_some());
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
}

#[test]
fn quiet_period_one_second_short_defers_and_exact_boundary_ingests() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("quiet.m4a", &m4a(CAPTURED, 1, b"owned audio"));
    let mut parts = fixture.parts();
    set_mtime(&source, parts.clock.now.secs - 29);
    let probe = GateRecorder::new(&fixture.recordings);

    let first = probed(&mut parts, &probe).expect("young source deferral");
    assert_eq!(first.deferred[0].reason, DeferralReason::NotAtRest);
    assert_eq!(probe.clones.get(), 0);

    set_mtime(&source, parts.clock.now.secs - 30);
    let second = probed(&mut parts, &probe).expect("exact quiet boundary");
    assert_eq!(second.ingested.len(), 1);
}

#[test]
fn stable_deferred_source_ingests_and_clears_deferral_state() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"owned payload");
    let source = fixture.add_recording("growing.m4a", &whole);
    let mut parts = fixture.parts();

    let previous = grown_source_snapshot(&parts, &source);
    parts.ledger.record_seen(&previous).unwrap();
    let report = parts.ingest().run(&Mode::default()).expect("stable source");
    assert_eq!(report.ingested.len(), 1);
    let seen = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(
        (
            seen.deferral_count,
            seen.deferral_reason,
            seen.deferred_size
        ),
        (0, None, None)
    );
}

fn grown_source_snapshot(parts: &Parts, source: &Path) -> SeenRow {
    let mut seen = partial_source_snapshot(parts, source);
    seen.deferral_count = 2;
    seen.deferral_reason = Some(DeferralReason::NotAtRest);
    seen
}

#[test]
fn grown_deferred_source_saves_complete_state_for_stable_retry() {
    let fixture = Fixture::new();
    let whole = m4a(CAPTURED, 1, b"owned payload");
    let source = fixture.add_recording("growing.m4a", &whole[..whole.len() - 12]);
    let mut parts = fixture.parts();
    let previous = partial_source_snapshot(&parts, &source);
    parts.ledger.record_seen(&previous).unwrap();
    fixture.add_recording("growing.m4a", &whole);
    let expected = grown_source_snapshot(&parts, &source);

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(
        report.deferred,
        vec![vpt_application::Deferred {
            path: source.clone(),
            reason: DeferralReason::NotAtRest,
        }]
    );
    assert!(report.ingested.is_empty());
    assert_eq!(parts.ledger.seen(&source).unwrap(), Some(expected));
}

#[test]
fn edited_source_waits_for_rest_then_becomes_a_second_recording() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("edited.m4a", &m4a(CAPTURED, 1, b"first take"));
    let mut parts = fixture.parts();
    let previous = first_take_snapshot(&parts, &source);
    let original = previous.recordings[0].id.clone();
    let archived = &previous.recordings[0].audio_path;
    std::fs::copy(&source, archived).unwrap();
    std::fs::set_permissions(archived, std::fs::Permissions::from_mode(0o600)).unwrap();
    parts.ledger.commit(&previous).unwrap();
    std::fs::write(&source, m4a(CAPTURED + 30, 2, b"second take")).unwrap();
    set_mtime(&source, parts.clock.now.secs - 5);

    let fresh = parts
        .ingest()
        .run(&Mode::default())
        .expect("recent edit defers");
    assert_eq!(fresh.deferred[0].reason, DeferralReason::NotAtRest);
    assert_eq!(parts.ledger.recordings().unwrap().len(), 1);
    parts.clock.now.secs += 60;
    let stable = parts
        .ingest()
        .run(&Mode::default())
        .expect("stable edit ingests");
    assert_eq!(stable.ingested.len(), 1);
    assert_ne!(stable.ingested[0].id, original);
    assert_eq!(parts.ledger.recordings().unwrap().len(), 2);
}

#[test]
fn unchanged_ingested_entry_refreshes_liveness_and_flags_without_clone() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("live.m4a", &m4a(CAPTURED, 1, b"owned audio"));
    let mut parts = fixture.parts();
    parts.ingest().run(&Mode::default()).unwrap();
    let mut previous = parts.ledger.seen(&source).unwrap().unwrap();
    previous.source_gone_at = Some(parts.clock.now);
    let first_seen = previous.first_seen;
    parts.ledger.record_seen(&previous).unwrap();
    parts.clock.now.secs += 1;
    let probe = GateRecorder::new(&fixture.recordings);
    probe.extra_flags.set(0x20);

    let report = probed(&mut parts, &probe).expect("liveness refresh");

    assert_eq!(report.skipped, 1);
    assert_eq!(probe.clones.get(), 0);
    let observed = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(observed.first_seen, first_seen);
    assert_eq!(observed.last_seen, parts.clock.now);
    assert_eq!(observed.flags & 0x20, 0x20);
    assert_eq!(observed.source_gone_at, None);
}
