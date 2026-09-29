mod support;

use std::cell::Cell;
use support::faults::DatalessRecorder;
use support::{CAPTURED, Fixture};
use vpt_adapters::{RootDir, VoiceMemosStore};
use vpt_application::ports::{Archive, RecorderStore, RecordingLedger, RunHistory, TitleOrigin};
use vpt_application::{Ingest, IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::history::{EventDetail, EventKind, EventState, SafeFailureClass};
use vpt_domain::run::{HistoryFact, RunOutcome};
use vpt_domain::time::UtcInstant;

#[test]
fn empty_and_invalid_once_sweeps_still_refresh_titles() {
    for missing in [None, Some("absent.m4a")] {
        let fixture = Fixture::new();
        fixture.add_title("a.m4a", "Alpha");
        let mut parts = fixture.parts();
        parts.store = VoiceMemosStore::open(&fixture.recordings)
            .unwrap()
            .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);
        let mode = Mode {
            dry_run: false,
            once: missing.map(|name| fixture.recordings.join(name)),
        };

        let outcome = parts.ingest().run(&mode);

        if let Some(name) = missing {
            assert_eq!(
                outcome.unwrap_err().failure,
                IngestFailure::OnceMissing(fixture.recordings.join(name))
            );
        } else {
            assert!(outcome.unwrap().ingested.is_empty());
        }
        assert!(fixture.state.join("title-copy/CloudRecordings.db").exists());
        assert!(
            parts
                .sink
                .receipts
                .first()
                .unwrap()
                .records
                .iter()
                .any(|record| matches!(record.fact, HistoryFact::RunStarted(_)))
        );
    }
}

#[test]
fn second_sweep_updates_liveness_in_history_transaction() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let first = parts.ingest().run(&Mode::default()).unwrap();
    let id = first.ingested[0].id.clone();
    let earlier = parts.ledger.seen(&source).unwrap().unwrap();
    parts.clock.now = UtcInstant {
        secs: CAPTURED + 7_200,
    };

    let later = parts.ingest().run(&Mode::default()).unwrap();

    assert!(later.ingested.is_empty());
    assert_eq!(later.skipped, 1);
    assert_eq!(later.already_ingested, vec![id.clone()]);
    assert_eq!(parts.ledger.recordings().unwrap().len(), 1);
    let current = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(current.first_seen, earlier.first_seen);
    assert_eq!(current.last_seen, parts.clock.now);
    let history = parts.ledger.recovery_view().unwrap();
    assert!(history.records.iter().any(|record| matches!(&record.fact, HistoryFact::WorkStep { recording: Some(recording), outcome: vpt_domain::run::WorkOutcome::Completed } if recording == &id && record.occurred_at == parts.clock.now)));
}

#[test]
fn once_selects_only_named_recording() {
    let fixture = Fixture::new();
    let selected = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    let other = fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
    let mut parts = fixture.parts();

    let report = parts
        .ingest()
        .run(&Mode {
            dry_run: false,
            once: Some(selected.clone()),
        })
        .unwrap();

    assert_eq!(report.ingested.len(), 1);
    assert_eq!(
        report.ingested[0].source_path.as_deref(),
        Some(selected.as_path())
    );
    assert!(parts.ledger.seen(&other).unwrap().is_none());
    assert_eq!(parts.ledger.recordings().unwrap().len(), 1);
}

#[test]
fn title_refresh_reads_fresh_copy_before_candidates() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    fixture.add_title("a.m4a", "Old title");
    let mut parts = fixture.parts();
    parts.store = VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);
    parts.store.refresh_titles().unwrap();
    let database = rusqlite::Connection::open(
        fixture
            .recordings
            .parent()
            .unwrap()
            .join("CloudRecordings.db"),
    )
    .unwrap();
    database
        .execute(
            "UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL = ?1",
            ["Fresh title"],
        )
        .unwrap();
    drop(database);

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.ingested[0].title.as_deref(), Some("Fresh title"));
    assert_eq!(report.ingested[0].title_source, TitleOrigin::VoiceMemos);
}

#[test]
fn ordinary_title_copy_failure_ingests_untitled_after_copy_attempt() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let source_database = fixture
        .recordings
        .parent()
        .unwrap()
        .join("CloudRecordings.db");
    std::fs::write(&source_database, b"not a sqlite database canary").unwrap();
    let mut parts = fixture.parts();
    parts.store = VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);
    assert!(!fixture.state.join("title-copy/CloudRecordings.db").exists());

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.ingested.len(), 1);
    assert_eq!(report.ingested[0].title, None);
    assert_eq!(report.ingested[0].title_source, TitleOrigin::Unavailable);
    assert!(fixture.state.join("title-copy/CloudRecordings.db").exists());
}

#[test]
fn unsafe_title_copy_refuses_without_candidate_work() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let outside = fixture.temp.path().join("owned-canary");
    std::fs::write(&outside, b"owned canary").unwrap();
    std::os::unix::fs::symlink(
        &outside,
        fixture
            .recordings
            .parent()
            .unwrap()
            .join("CloudRecordings.db"),
    )
    .unwrap();
    let mut parts = fixture.parts();
    parts.store = VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&RootDir::open(&fixture.state).unwrap(), true);

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::PathEscape(_)));
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert_eq!(std::fs::read(&outside).unwrap(), b"owned canary");
    let history = parts.ledger.recovery_view().unwrap();
    assert!(history.records.iter().any(
        |record| matches!(record.fact, HistoryFact::Event(ref event)
        if event.kind == EventKind::IngestFailed
            && event.state == EventState::Failed
            && event.detail == EventDetail::Failure(SafeFailureClass::ConfigurationRefused)
            && event.paths.is_empty())
    ));
    assert!(
        history
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunFinished(RunOutcome::Failed)))
    );
}

#[test]
fn dataless_candidate_is_deferred_without_opening() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let store = DatalessRecorder {
        base: VoiceMemosStore::open(&fixture.recordings).unwrap(),
        opens: Cell::new(0),
    };
    let mut ingest = Ingest {
        recorder: &store,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let report = ingest.run(&Mode::default()).unwrap();

    assert_eq!(report.deferred.len(), 1);
    assert_eq!(
        report.deferred[0].reason,
        vpt_domain::sweep::DeferralReason::Dataless
    );
    assert_eq!(store.opens.get(), 0);
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
}
