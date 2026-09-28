mod support;

use support::faults::{ArchiveFault, FaultArchive, FaultLedger, HistoryFault};
use support::{CAPTURED, Fixture};
use vpt_application::ports::{Archive, RecordingLedger, RunHistory, TrashError};
use vpt_application::{Ingest, IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::history::{EventDetail, EventKind, SafeFailureClass};
use vpt_domain::run::{HistoryFact, RunOutcome};

#[test]
fn run_start_failure_stops_without_candidate_or_cleanup() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::Begin);
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    assert_eq!(ledger.failed_calls.get(), 1);
    assert!(ledger.base.recovery_view().unwrap().records.is_empty());
    assert!(parts.sink.receipts.is_empty());
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
}

#[test]
fn owned_staging_is_only_trash_target_after_prepublication_failure() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let archive = FaultArchive {
        base: parts.archive,
        fault: ArchiveFault::StageOwned,
    };
    parts.sink.watched_audio = Some(fixture.audio.clone());
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert_eq!(error.failure, IngestFailure::NoSpace);
    let attempts = parts.trash.attempted.borrow();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].parent(), Some(fixture.audio.as_path()));
    assert!(
        attempts[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".vpt-staging-")
    );
    assert_eq!(*parts.trash.moved.borrow(), *attempts);
    assert!(archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.ledger.recordings().unwrap().is_empty());
}

#[test]
fn invalid_staged_container_cleanup_waits_for_failure_history() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", b"invalid owned audio bytes");
    let mut parts = fixture.parts();
    parts.sink.watched_audio = Some(fixture.audio.clone());

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::Archive(_)));
    assert_eq!(parts.sink.owned_stage_present_at_failure_event, Some(true));
    assert_eq!(parts.trash.attempted.borrow().len(), 1);
    assert!(parts.archive.staged_leftovers().unwrap().is_empty());
    let history = parts.ledger.recovery_view().unwrap();
    assert!(
        history
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunFinished(RunOutcome::Failed)))
    );
}

#[test]
fn placed_unsynced_target_is_retained_for_recovery() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let archive = FaultArchive {
        base: parts.archive,
        fault: ArchiveFault::PlacedUnsynced,
    };
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::Sync(_)));
    assert_eq!(archive.archived().unwrap().len(), 1);
    assert!(archive.staged_leftovers().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(parts.ledger.recordings().unwrap().is_empty());
    assert!(
        parts
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::Event(_)))
    );
}

#[test]
fn published_target_survives_unconfirmed_state_commit() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
    let mut parts = fixture.parts();
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::RecordUnconfirmed);
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    assert!(error.completed.is_empty());
    assert!(error.known_receipt.is_none());
    assert_eq!(parts.archive.archived().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(ledger.base.recordings().unwrap().is_empty());
    assert_eq!(ledger.failed_calls.get(), 1);
}

#[test]
fn postcommit_receipt_preserves_completed_identity_on_failure() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
    let mut parts = fixture.parts();
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::RecordPostCommit);
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    let receipt = error
        .known_receipt
        .as_ref()
        .expect("known post-commit receipt");
    assert_eq!(receipt.completed.len(), 1);
    assert_eq!(error.completed, receipt.completed);
    assert!(ledger.base.by_id(&receipt.completed[0]).unwrap().is_some());
    assert_eq!(parts.archive.archived().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
}

#[test]
fn required_intent_history_failure_stops_before_publish_and_cleanup() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
    fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
    let mut parts = fixture.parts();
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::Intent);
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert!(parts.archive.archived().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
    assert_eq!(ledger.failed_calls.get(), 1);
    assert!(ledger.base.recordings().unwrap().is_empty());
}

#[test]
fn required_failure_event_save_never_recurses_or_cleans_up() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", b"invalid owned audio bytes");
    let mut parts = fixture.parts();
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::FailureEvent);
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    assert_eq!(ledger.failed_calls.get(), 1);
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(
        !ledger
            .base
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::Event(_)))
    );
}

#[test]
fn cleanup_helper_error_keeps_owned_staging_and_safe_diagnostic() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", b"invalid owned audio bytes");
    let mut parts = fixture.parts();
    *parts.trash.failure.borrow_mut() = Some(TrashError::Failed("private helper canary".into()));

    let error = parts.ingest().run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::Archive(_)));
    assert_eq!(parts.trash.attempted.borrow().len(), 1);
    assert!(parts.trash.moved.borrow().is_empty());
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert!(!error.log.join(" ").contains("private helper canary"));
    assert!(error.log.join(" ").contains("cleanup pending"));
}

#[test]
fn non_owned_or_placed_target_never_reaches_trash() {
    for fault in [ArchiveFault::StageUnowned, ArchiveFault::PlacedUnsynced] {
        let fixture = Fixture::new();
        fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
        let mut parts = fixture.parts();
        let archive = FaultArchive {
            base: parts.archive,
            fault,
        };
        let mut ingest = Ingest {
            recorder: &parts.store,
            archive: &archive,
            ledger: &parts.ledger,
            clock: &parts.clock,
            trash: &parts.trash,
            sink: &mut parts.sink,
            settings: &parts.settings,
        };

        let _ = ingest.run(&Mode::default()).unwrap_err();

        assert!(parts.trash.attempted.borrow().is_empty());
        assert!(parts.trash.moved.borrow().is_empty());
        if fault == ArchiveFault::PlacedUnsynced {
            assert_eq!(archive.archived().unwrap().len(), 1);
        } else {
            assert!(archive.archived().unwrap().is_empty());
        }
    }
}

#[test]
fn archive_escape_after_stage_saves_safe_failure_without_cleanup() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    let archive = FaultArchive {
        base: parts.archive,
        fault: ArchiveFault::EscapeBeforePublish,
    };
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &archive,
        ledger: &parts.ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::PathEscape(_)));
    assert_eq!(archive.staged_leftovers().unwrap().len(), 1);
    assert!(archive.archived().unwrap().is_empty());
    assert!(parts.trash.attempted.borrow().is_empty());
    let history = parts.ledger.recovery_view().unwrap();
    assert!(history.records.iter().any(
        |record| matches!(record.fact, HistoryFact::Event(ref event)
        if event.kind == EventKind::IngestFailed
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
