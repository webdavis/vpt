mod support;

use std::os::unix::fs::PermissionsExt;
use support::{CAPTURED, Fixture};
use vpt_application::ports::{
    Archive, Clock, RecordingLedger, RunHistory, StageStates, TitleOrigin,
};
use vpt_application::{Ingest, IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::run::{EffectKind, EffectOutcome, HistoryFact, RunOutcome, WorkOutcome};
use vpt_domain::time::{UtcInstant, UtcOffset};

#[test]
fn whole_recording_archives_bytes_mode_and_identity() {
    let fixture = Fixture::new();
    let bytes = m4a(CAPTURED, 612, b"owned synthetic audio");
    let source = fixture.add_recording("20260824 144736-4F3AB19C.m4a", &bytes);
    let mut parts = fixture.parts();

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.ingested.len(), 1);
    let record = &report.ingested[0];
    assert!(record.id.as_str().starts_with("2026-08-24T144736-"));
    assert_eq!(record.source_path.as_deref(), Some(source.as_path()));
    assert_eq!(record.captured_at, UtcInstant { secs: CAPTURED });
    assert_eq!(record.captured_offset, parts.clock.offset);
    assert_eq!(record.duration_secs, 612);
    assert_eq!(record.title, None);
    assert_eq!(record.title_source, TitleOrigin::Unavailable);
    assert_eq!(record.stages, StageStates::fresh());
    assert_eq!(
        record.audio_path,
        fixture.audio.join(format!("{}.m4a", record.id))
    );
    assert_eq!(std::fs::read(&record.audio_path).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&record.audio_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        parts.ledger.by_id(&record.id).unwrap(),
        Some(record.clone())
    );
    let seen = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(seen.recording, Some(record.id.clone()));
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.trash.moved.borrow().is_empty());
}

#[test]
fn run_start_is_committed_and_exported_before_title_refresh() {
    let fixture = Fixture::new();
    fixture.add_title("a.m4a", "Alpha");
    let mut parts = fixture.parts();
    parts.store = vpt_adapters::VoiceMemosStore::open(&fixture.recordings)
        .unwrap()
        .with_titles_root(&vpt_adapters::RootDir::open(&fixture.state).unwrap(), true);
    parts.sink.title_copy = Some(fixture.state.join("title-copy"));

    let _ = parts.ingest().run(&Mode::default());

    assert!(matches!(
        parts
            .sink
            .receipts
            .first()
            .and_then(|receipt| receipt.records.first())
            .map(|record| &record.fact),
        Some(HistoryFact::RunStarted(vpt_domain::run::RunKind::Ingest))
    ));
    assert_eq!(parts.sink.title_absent_at_start, Some(true));
    assert!(fixture.state.join("title-copy").exists());
}

#[test]
fn dry_run_refuses_before_any_port_or_history_effect() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let before = std::fs::read_dir(&fixture.audio).unwrap().count();

    struct PanicClock;
    impl Clock for PanicClock {
        fn now(&self) -> UtcInstant {
            panic!("dry run consulted the clock")
        }
        fn offset_at(&self, _: UtcInstant) -> UtcOffset {
            panic!("dry run consulted the clock")
        }
    }
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &PanicClock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };
    let error = ingest
        .run(&Mode {
            dry_run: true,
            once: None,
        })
        .unwrap_err();

    assert_eq!(error.failure, IngestFailure::DryRunUnavailable);
    assert!(parts.sink.receipts.is_empty());
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert_eq!(std::fs::read_dir(&fixture.audio).unwrap().count(), before);
    assert!(parts.trash.moved.borrow().is_empty());
}

#[test]
fn archive_intent_is_exported_before_publication() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    parts.sink.watched_audio = Some(fixture.audio.clone());

    let _ = parts.ingest().run(&Mode::default());

    assert_eq!(parts.sink.intent_before_publication, Some(true));
    let intent = parts
        .sink
        .receipts
        .iter()
        .position(|receipt| {
            receipt.records.iter().any(|record| {
                matches!(
                    record.fact,
                    HistoryFact::EffectIntent {
                        kind: EffectKind::Archive,
                        ..
                    }
                )
            })
        })
        .unwrap();
    let completed = parts
        .sink
        .receipts
        .iter()
        .position(|receipt| {
            receipt.records.iter().any(|record| {
                matches!(
                    record.fact,
                    HistoryFact::EffectObserved {
                        outcome: EffectOutcome::Completed,
                        ..
                    }
                )
            })
        })
        .unwrap();
    assert!(intent < completed);
}

#[test]
fn whole_recording_commits_seen_record_and_outcome_together() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();

    let _ = parts.ingest().run(&Mode::default());

    let receipt = parts
        .sink
        .receipts
        .iter()
        .find(|receipt| {
            receipt.records.iter().any(|record| {
                matches!(
                    record.fact,
                    HistoryFact::WorkStep {
                        outcome: WorkOutcome::Completed,
                        ..
                    }
                )
            })
        })
        .expect("completed work receipt");
    assert_eq!(receipt.completed.len(), 1);
    let id = &receipt.completed[0];
    assert_eq!(
        parts
            .ledger
            .seen(&source)
            .unwrap()
            .unwrap()
            .recording
            .as_ref(),
        Some(id)
    );
    assert!(parts.ledger.by_id(id).unwrap().is_some());
    assert!(receipt.records.iter().any(|record| matches!(
        record.fact,
        HistoryFact::EffectObserved {
            outcome: EffectOutcome::Completed,
            ..
        }
    )));
}

#[test]
fn success_finishes_run_without_success_event() {
    let fixture = Fixture::new();
    let mut parts = fixture.parts();

    let _ = parts.ingest().run(&Mode::default());

    let history = parts.ledger.recovery_view().unwrap();
    assert!(
        history
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunFinished(RunOutcome::Completed)))
    );
    assert!(
        !history
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::Event(_)))
    );
}
