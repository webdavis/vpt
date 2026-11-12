mod support;

use support::{CAPTURED, Fixture};
use vpt_application::ports::{RecordingLedger, RunHistory};
use vpt_application::{IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::history::EventKind;
use vpt_domain::run::HistoryFact;

#[test]
fn vanished_source_is_marked_once_and_archive_is_preserved() {
    let fixture = Fixture::new();
    let gone = fixture.add_recording("gone.m4a", &m4a(CAPTURED, 1, b"gone"));
    fixture.add_recording("remaining.m4a", &m4a(CAPTURED + 1, 1, b"remaining"));
    let mut parts = fixture.parts();
    parts.ingest().run(&Mode::default()).unwrap();
    let id = parts
        .ledger
        .seen(&gone)
        .unwrap()
        .unwrap()
        .recording
        .unwrap();
    let archived = parts.ledger.by_id(&id).unwrap().unwrap().audio_path;
    std::fs::remove_file(&gone).unwrap();
    parts.clock.now.secs += 100;
    let first_absent = parts.clock.now;

    parts.ingest().run(&Mode::default()).unwrap();
    assert_eq!(
        parts.ledger.seen(&gone).unwrap().unwrap().source_gone_at,
        Some(first_absent)
    );
    parts.clock.now.secs += 100;
    parts.ingest().run(&Mode::default()).unwrap();
    assert_eq!(
        parts.ledger.seen(&gone).unwrap().unwrap().source_gone_at,
        Some(first_absent)
    );
    assert!(archived.exists());
}

#[test]
fn previously_populated_empty_source_aborts_once_with_no_archive_removal() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("gone.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let archived = parts.ingest().run(&Mode::default()).unwrap().ingested[0]
        .audio_path
        .clone();
    std::fs::remove_file(source).unwrap();

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert_eq!(error.failure, IngestFailure::EmptyStore);
    assert!(archived.exists());
    let history = parts.ledger.recovery_view().unwrap();
    assert_eq!(
        history
            .records
            .iter()
            .filter(|record| matches!(&record.fact,
        HistoryFact::Event(event) if event.kind == EventKind::IngestFailed))
            .count(),
        1
    );
}

#[test]
fn once_and_dry_run_do_not_mark_other_sources_gone() {
    let fixture = Fixture::new();
    let gone = fixture.add_recording("gone.m4a", &m4a(CAPTURED, 1, b"gone"));
    let remaining = fixture.add_recording("remaining.m4a", &m4a(CAPTURED + 1, 1, b"remaining"));
    let mut parts = fixture.parts();
    parts.ingest().run(&Mode::default()).unwrap();
    std::fs::remove_file(&gone).unwrap();
    for mode in [
        Mode {
            dry_run: true,
            once: None,
            import: None,
        },
        Mode {
            dry_run: false,
            once: Some(remaining),
            import: None,
        },
    ] {
        parts.ingest().run(&mode).unwrap();
        assert_eq!(
            parts.ledger.seen(&gone).unwrap().unwrap().source_gone_at,
            None
        );
    }
}

#[test]
fn empty_source_dry_run_refuses_without_saving_history() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("gone.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    parts.ingest().run(&Mode::default()).unwrap();
    std::fs::remove_file(source).unwrap();
    let before = parts.ledger.recovery_view().unwrap().records;
    let error = parts
        .ingest()
        .run(&Mode {
            dry_run: true,
            once: None,
            import: None,
        })
        .unwrap_err();
    assert_eq!(error.failure, IngestFailure::EmptyStore);
    assert_eq!(parts.ledger.recovery_view().unwrap().records, before);
}
