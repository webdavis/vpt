mod support;

use support::{CAPTURED, Fixture};
use vpt_adapters::{RootDir, VoiceMemosStore};
use vpt_application::Mode;
use vpt_application::ports::{Archive, RecordingLedger, RunHistory, TitleOrigin};
use vpt_domain::fixtures::m4a;
use vpt_domain::sweep::DeferralReason;

fn dry_run() -> Mode {
    Mode {
        dry_run: true,
        once: None,
        import: None,
    }
}

#[test]
fn observation_defers_capture_times_that_cannot_form_an_identity() {
    use vpt_domain::fixtures::{box_of, mvhd_v1};
    let fixture = Fixture::new();
    let bytes = box_of(b"moov", &mvhd_v1(253_402_387_200, 1));
    fixture.add_recording("future.m4a", &bytes);
    let mut parts = fixture.parts();
    let report = parts.ingest().run(&dry_run()).unwrap();
    assert!(report.would_ingest.is_empty());
    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.ledger.recovery_view().unwrap().records.is_empty());
}

#[test]
fn observation_runs_source_gates_without_staging_history_or_title_copy() {
    let fixture = Fixture::new();
    let good = fixture.add_recording("good.m4a", &m4a(CAPTURED, 1, b"audio"));
    let bad = fixture.add_recording("bad.m4a", b"incomplete");
    fixture.add_title("good.m4a", "Private live title");
    let mut parts = fixture.parts();
    parts.store = VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);

    let report = parts.ingest().run(&dry_run()).unwrap();

    assert_eq!(report.would_ingest.len(), 1);
    assert_eq!(report.would_ingest[0].path, good);
    assert_eq!(report.would_ingest[0].title, None);
    assert_eq!(
        report.would_ingest[0].title_source,
        TitleOrigin::Unavailable
    );
    assert_eq!(report.deferred[0].path, bad);
    assert_eq!(report.deferred[0].reason, DeferralReason::InvalidContainer);
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert!(parts.ledger.seen_all().unwrap().is_empty());
    assert!(parts.ledger.recovery_view().unwrap().records.is_empty());
    assert!(parts.sink.receipts.is_empty());
    assert!(parts.archive.archived().unwrap().is_empty());
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(!fixture.state.join("title-copy").exists());
}

#[test]
fn observation_preserves_known_title_and_liveness() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    fixture.add_title("a.m4a", "Accepted title");
    let mut parts = fixture.parts();
    parts.store = VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);
    let first = parts.ingest().run(&Mode::default()).unwrap();
    let seen = parts.ledger.seen(&path).unwrap();
    let history = parts.ledger.recovery_view().unwrap().records;
    parts.clock.now.secs += 100;

    let report = parts.ingest().run(&dry_run()).unwrap();

    assert_eq!(report.skipped, 1);
    assert_eq!(report.already_ingested, vec![first.ingested[0].id.clone()]);
    assert_eq!(parts.ledger.seen(&path).unwrap(), seen);
    assert_eq!(parts.ledger.recovery_view().unwrap().records, history);
}

#[test]
fn observation_enforces_size_and_quiet_period_and_once_selection() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    fixture.add_recording("ignored.m4a", b"bad");
    let mut parts = fixture.parts();
    let mode = Mode {
        dry_run: true,
        once: Some(path),
        import: None,
    };
    parts.settings.max_audio_bytes = 1;
    let report = parts.ingest().run(&mode).unwrap();
    assert_eq!(report.deferred.len(), 1);
    assert_eq!(report.deferred[0].reason, DeferralReason::AudioTooLarge);
    parts.settings.max_audio_bytes = 1_000;
    parts.clock.now.secs = CAPTURED;
    let report = parts.ingest().run(&mode).unwrap();
    assert_eq!(report.deferred[0].reason, DeferralReason::NotAtRest);
    assert!(parts.ledger.seen_all().unwrap().is_empty());
}

#[test]
fn observation_uses_digest_for_known_titles_after_renames_and_edits() {
    let fixture = Fixture::new();
    let path = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    fixture.add_title("a.m4a", "Accepted title");
    let mut parts = fixture.parts();
    parts.store = VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);
    parts.ingest().run(&Mode::default()).unwrap();
    let renamed = fixture.recordings.join("renamed.m4a");
    std::fs::rename(path, &renamed).unwrap();
    let report = parts.ingest().run(&dry_run()).unwrap();
    assert_eq!(
        report.would_ingest[0].title.as_deref(),
        Some("Accepted title")
    );
    std::fs::rename(renamed, fixture.recordings.join("a.m4a")).unwrap();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 2, b"edited audio"));
    let report = parts.ingest().run(&dry_run()).unwrap();
    assert_eq!(report.would_ingest[0].title, None);
    assert_eq!(
        report.would_ingest[0].title_source,
        TitleOrigin::Unavailable
    );
}
