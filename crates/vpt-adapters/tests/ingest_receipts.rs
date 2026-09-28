mod support;

use std::cell::Cell;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use support::faults::{FaultLedger, HistoryFault};
use support::{CAPTURED, Fixture, StopAt};
use vpt_adapters::{LoggedEvents, RootDir};
use vpt_application::ports::{
    Archive, Clock, EventSink, HistoryWrite, RecordingLedger, RunHistory, SinkOutcome, SinkWarning,
};
use vpt_application::{Ingest, IngestFailure, Mode};
use vpt_domain::fixtures::m4a;
use vpt_domain::history::{EventDetail, EventKind, EventState, SafeFailureClass};
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};
use vpt_domain::time::{UtcInstant, UtcOffset};

#[test]
fn fault_ledger_preserves_atomic_recovery_through_logged_events() {
    let fixture = Fixture::new();
    std::fs::set_permissions(&fixture.state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let parts = fixture.parts();
    let history = FaultLedger::new(parts.ledger, HistoryFault::Intent);
    let HistoryWrite::Committed(started) =
        history.begin_run(RunKind::Ingest, UtcInstant { secs: 10 })
    else {
        panic!("fixture run was not committed");
    };
    let before = history.recovery_view().unwrap();
    let root = RootDir::open(&fixture.state).unwrap();
    let mut sink = LoggedEvents::new(&root, &history, &[], &[], || UtcInstant { secs: 20 });

    assert_eq!(
        sink.recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new(),
        }
    );
    let after = history.recovery_view().unwrap();
    let saved = &after.records[before.records.len()..];
    assert_eq!(saved.len(), 4);
    assert_eq!(saved[0].fact, HistoryFact::RunStarted(RunKind::Recover));
    let HistoryFact::RecoveryProgress { baseline, .. } = &saved[1].fact else {
        panic!("atomic recovery progress missing");
    };
    assert_eq!(baseline.generation, None);
    assert_eq!(baseline.pending_from, Some(started.records[0].id));
    assert_eq!(baseline.through, Some(started.records[0].id));
    assert!(matches!(saved[2].fact, HistoryFact::RecoverySummary { .. }));
    assert_eq!(
        saved[3].fact,
        HistoryFact::RunFinished(RunOutcome::Completed)
    );
    assert!(saved.iter().all(|record| record.run == saved[0].run));
    assert!(history.pending_exports().unwrap().is_empty());
    assert_eq!(history.failed_calls.get(), 0);
}

#[test]
fn sink_stop_after_start_intent_or_record_stops_new_work() {
    for phase in [StopAt::Start, StopAt::Intent, StopAt::Work] {
        let fixture = Fixture::new();
        fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"a"));
        let second = fixture.add_recording("b.m4a", &m4a(CAPTURED + 1, 1, b"b"));
        let mut parts = fixture.parts();
        parts.sink.stop_at = Some(phase);

        let error = parts.ingest().run(&Mode::default()).unwrap_err();

        assert!(matches!(error.failure, IngestFailure::SinkStopped(_)));
        let receipt = error
            .known_receipt
            .as_ref()
            .expect("known sink stop receipt");
        assert_eq!(error.completed, receipt.completed);
        assert!(parts.ledger.seen(&second).unwrap().is_none());
        assert!(parts.trash.attempted.borrow().is_empty());
        match phase {
            StopAt::Start => {
                assert!(parts.archive.staged_leftovers().unwrap().is_empty());
                assert!(parts.archive.archived().unwrap().is_empty());
            }
            StopAt::Intent => {
                assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
                assert!(parts.archive.archived().unwrap().is_empty());
            }
            StopAt::Work => {
                assert!(parts.archive.staged_leftovers().unwrap().is_empty());
                assert_eq!(parts.archive.archived().unwrap().len(), 1);
                assert_eq!(receipt.completed.len(), 1);
            }
        }
    }
}

#[test]
fn export_only_warnings_preserve_work_and_finish() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    parts.sink.warnings.insert(SinkWarning::ExportUnavailable);

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(report.ingested.len(), 1);
    assert_eq!(
        report.warnings,
        BTreeSet::from([SinkWarning::ExportUnavailable])
    );
    assert_eq!(parts.ledger.recordings().unwrap().len(), 1);
    assert!(
        parts
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(record.fact, HistoryFact::RunFinished(RunOutcome::Completed)))
    );
}

#[test]
fn warnings_are_deduplicated_bounded_and_content_free() {
    let fixture = Fixture::new();
    fixture.add_recording("a.m4a", &m4a(CAPTURED, 1, b"audio"));
    let mut parts = fixture.parts();
    parts.sink.warnings = BTreeSet::from([
        SinkWarning::ExportUnavailable,
        SinkWarning::ExportProgressUnconfirmed,
        SinkWarning::ExportFailureEvidenceUnconfirmed,
        SinkWarning::ExportBookkeepingPostCommitFailure,
        SinkWarning::HookFailed,
    ]);

    let report = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(parts.sink.receipts.len(), 4);
    assert_eq!(report.warnings, parts.sink.warnings);
    assert_eq!(report.warnings.len(), 5);
    assert!(report.log.is_empty());
}

struct TickClock(Cell<i64>);

impl Clock for TickClock {
    fn now(&self) -> UtcInstant {
        let tick = self.0.get();
        self.0.set(tick + 1);
        UtcInstant {
            secs: CAPTURED + 3_600 + tick,
        }
    }

    fn offset_at(&self, _: UtcInstant) -> UtcOffset {
        UtcOffset { secs: -21_600 }
    }
}

#[test]
fn ordinary_failure_saves_one_safe_event_with_original_time() {
    let fixture = Fixture::new();
    fixture.add_recording("private-name.m4a", b"private invalid media canary");
    let mut parts = fixture.parts();
    let clock = TickClock(Cell::new(0));
    let mut ingest = Ingest {
        recorder: &parts.store,
        archive: &parts.archive,
        ledger: &parts.ledger,
        clock: &clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    };

    let error = ingest.run(&Mode::default()).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::Archive(_)));
    let history = parts.ledger.recovery_view().unwrap();
    let events: Vec<_> = history
        .records
        .iter()
        .filter_map(|record| match &record.fact {
            HistoryFact::Event(event) => Some((record, event)),
            _ => None,
        })
        .collect();
    assert_eq!(events.len(), 1);
    let (saved, event) = events[0];
    let occurred = UtcInstant {
        secs: CAPTURED + 3_601,
    };
    assert_eq!(saved.occurred_at, occurred);
    assert_eq!(event.occurred_at, occurred);
    assert_eq!(event.kind, EventKind::IngestFailed);
    assert_eq!(event.state, EventState::Failed);
    assert_eq!(
        event.detail,
        EventDetail::Failure(SafeFailureClass::ArchiveFailed)
    );
    assert!(event.paths.is_empty());
    assert!(event.recording.is_none());
    assert!(!format!("{event:?}").contains("private-name"));
    assert!(!format!("{event:?}").contains("private invalid media"));
    assert!(
        history
            .records
            .iter()
            .any(|record| record.occurred_at == occurred
                && matches!(record.fact, HistoryFact::RunFinished(RunOutcome::Failed)))
    );
}
