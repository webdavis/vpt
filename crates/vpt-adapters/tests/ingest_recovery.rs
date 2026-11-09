mod support;

use support::{CAPTURED, Fixture};
use vpt_adapters::digest_open;
use vpt_application::ports::{Archive, RecordingLedger};
use vpt_application::{IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

#[test]
fn known_digest_reuses_identity_even_when_new_timezone_would_exceed_identity_year_range() {
    use vpt_domain::fixtures::{box_of, mvhd_v1};
    let fixture = Fixture::new();
    let bytes = box_of(b"moov", &mvhd_v1(253_402_300_799, 1));
    let original = fixture.add_recording("boundary.m4a", &bytes);
    let mut parts = fixture.parts();
    parts.clock.offset = UtcOffset { secs: 0 };
    let first = parts
        .ingest()
        .run(&Mode::default())
        .unwrap()
        .ingested
        .remove(0);
    std::fs::rename(original, fixture.recordings.join("renamed.m4a")).unwrap();
    parts.clock.offset = UtcOffset { secs: 3_600 };
    let report = parts.ingest().run(&Mode::default()).unwrap();
    assert_eq!(report.already_ingested, vec![first.id]);
    assert!(report.deferred.is_empty());
}

#[test]
fn renamed_source_after_timezone_change_keeps_identity_and_archive() {
    let fixture = Fixture::new();
    let original = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let first = parts
        .ingest()
        .run(&Mode::default())
        .unwrap()
        .ingested
        .remove(0);
    let renamed = fixture.recordings.join("renamed.m4a");
    std::fs::rename(&original, &renamed).unwrap();
    parts.clock.offset = UtcOffset { secs: 36_000 };

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert!(report.ingested.is_empty());
    assert_eq!(report.already_ingested, vec![first.id.clone()]);
    let records = parts.ledger.recordings().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, first.id);
    assert_eq!(records[0].audio_path, first.audio_path);
    assert_eq!(records[0].source_path.as_ref(), Some(&renamed));
    assert_eq!(
        parts.ledger.seen(&renamed).unwrap().unwrap().recording,
        Some(first.id)
    );
    assert_eq!(parts.archive.archived().unwrap().len(), 1);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert_eq!(parts.trash.moved.borrow().len(), 1);
}

#[test]
fn orphan_with_missing_source_recovers_archived_timezone_and_instant() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 23, b"orphan");
    let path = fixture.add_recording("a.m4a", &bytes);
    let digest = digest_open(&mut std::fs::File::open(&path).unwrap()).unwrap();
    let offset = UtcOffset { secs: -21_600 };
    let id = RecordingId::derive(UtcInstant { secs: CAPTURED }, offset, &digest).unwrap();
    let archived = fixture.audio.join(format!("{id}.m4a"));
    std::fs::rename(&path, &archived).unwrap();
    let mut parts = fixture.parts();
    parts.clock.offset = UtcOffset { secs: 36_000 };

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.recovered, vec![id.clone()]);
    let recovered = parts.ledger.by_id(&id).unwrap().unwrap();
    assert_eq!(recovered.captured_at.secs, CAPTURED);
    assert_eq!(recovered.captured_offset, offset);
    assert_eq!(recovered.duration_secs, 23);
    assert_eq!(recovered.digest, digest);
    assert_eq!(recovered.source_path, None);
    assert_eq!(recovered.audio_path, archived);
    assert_eq!(std::fs::read(archived).unwrap(), bytes);
}

#[test]
fn matching_existing_archive_recovers_rows_and_trashes_redundant_stage() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let digest = digest_open(&mut std::fs::File::open(&path).unwrap()).unwrap();
    let mut parts = fixture.parts();
    let id =
        RecordingId::derive(UtcInstant { secs: CAPTURED }, parts.clock.offset, &digest).unwrap();
    std::fs::copy(&path, fixture.audio.join(format!("{id}.m4a"))).unwrap();

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.already_ingested, vec![id.clone()]);
    assert_eq!(
        parts.ledger.by_id(&id).unwrap().unwrap().source_path,
        Some(path)
    );
    assert_eq!(parts.ledger.recordings().unwrap().len(), 1);
    assert_eq!(parts.trash.moved.borrow().len(), 1);
}

#[test]
fn different_archive_content_is_preserved_and_refused_as_collision() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let digest = digest_open(&mut std::fs::File::open(&path).unwrap()).unwrap();
    let mut parts = fixture.parts();
    let id =
        RecordingId::derive(UtcInstant { secs: CAPTURED }, parts.clock.offset, &digest).unwrap();
    let archived = fixture.audio.join(format!("{id}.m4a"));
    std::fs::write(&archived, b"foreign content").unwrap();

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(
        error.failure,
        IngestFailure::ArchiveCollision { .. }
    ));
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert_eq!(std::fs::read(archived).unwrap(), b"foreign content");
}

#[test]
fn later_collision_preserves_committed_renamed_source_identity_in_failure() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let original = parts
        .ingest()
        .run(&Mode::default())
        .unwrap()
        .ingested
        .remove(0);
    std::fs::rename(path, fixture.recordings.join("b.m4a")).unwrap();
    let later = fixture.add_recording("z.m4a", &m4a(CAPTURED + 10, 1, b"later"));
    let digest = digest_open(&mut std::fs::File::open(&later).unwrap()).unwrap();
    let id = RecordingId::derive(
        UtcInstant {
            secs: CAPTURED + 10,
        },
        parts.clock.offset,
        &digest,
    )
    .unwrap();
    std::fs::write(fixture.audio.join(format!("{id}.m4a")), b"foreign").unwrap();

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(
        error.failure,
        IngestFailure::ArchiveCollision { .. }
    ));
    assert!(error.completed.contains(&original.id));
    assert_eq!(
        parts
            .ledger
            .by_id(&original.id)
            .unwrap()
            .unwrap()
            .source_path,
        Some(fixture.recordings.join("b.m4a"))
    );
}

#[test]
fn changes_after_digest_verification_prevent_recovery_and_source_reassociation() {
    use support::faults::{ArchiveFault, FaultArchive};
    use vpt_application::Ingest;
    for known in [false, true] {
        let fixture = Fixture::new();
        let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
        let mut parts = fixture.parts();
        let digest = digest_open(&mut std::fs::File::open(&source).unwrap()).unwrap();
        let id = RecordingId::derive(UtcInstant { secs: CAPTURED }, parts.clock.offset, &digest)
            .unwrap();
        let archived = fixture.audio.join(format!("{id}.m4a"));
        let renamed = source.with_file_name("renamed.m4a");
        if known {
            parts.ingest().run(&Mode::default()).unwrap();
            std::fs::rename(&source, &renamed).unwrap();
        } else {
            std::fs::copy(&source, &archived).unwrap();
        }
        let archive = FaultArchive {
            base: parts.archive,
            fault: ArchiveFault::ChangedBeforeExistingSync,
        };
        let error = Ingest {
            recorder: &parts.store,
            archive: &archive,
            ledger: &parts.ledger,
            clock: &parts.clock,
            trash: &parts.trash,
            sink: &mut parts.sink,
            settings: &parts.settings,
        }
        .run(&Mode::default())
        .unwrap_err();
        assert!(
            matches!(error.failure, IngestFailure::Sync(_)),
            "{:?}",
            error.failure
        );
        assert_eq!(
            std::fs::read(archived).unwrap(),
            b"changed before archive sync"
        );
        let rows = parts.ledger.recordings().unwrap();
        assert_eq!(rows.len(), usize::from(known));
        if known {
            assert_eq!(rows[0].source_path.as_ref(), Some(&source));
            assert_eq!(rows[0].digest, digest);
            assert!(parts.ledger.seen(&renamed).unwrap().is_none());
        }
    }
}

#[test]
fn orphan_recovery_never_combines_one_files_container_with_another_files_digest() {
    use support::faults::{ArchiveFault, FaultArchive};
    use vpt_application::Ingest;
    let fixture = Fixture::new();
    let replacement = m4a(CAPTURED + 3_600, 2, b"replacement audio");
    let source = fixture.add_recording("replacement.m4a", &replacement);
    let digest = digest_open(&mut std::fs::File::open(&source).unwrap()).unwrap();
    let mut parts = fixture.parts();
    let id = RecordingId::derive(
        UtcInstant {
            secs: CAPTURED + 3_600,
        },
        parts.clock.offset,
        &digest,
    )
    .unwrap();
    let path = fixture.audio.join(format!("{id}.m4a"));
    std::fs::rename(source, path.with_extension("replacement")).unwrap();
    std::fs::write(&path, m4a(CAPTURED, 1, b"original audio")).unwrap();
    let archive = FaultArchive {
        base: parts.archive,
        fault: ArchiveFault::ReplacedAfterOpen,
    };
    let error = Ingest {
        recorder: &parts.store,
        archive: &archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default())
    .unwrap_err();
    assert!(matches!(error.failure, IngestFailure::Sync(_)));
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert_eq!(std::fs::read(path).unwrap(), replacement);
}
