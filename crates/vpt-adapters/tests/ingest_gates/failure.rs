use super::*;
use support::faults::{FaultLedger, HistoryFault};

fn staged_fault(fault: HistoryFault) -> (Fixture, Parts) {
    let fixture = Fixture::new();
    fixture.add_recording("stage.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let parts = fixture.parts();
    assert!(matches!(
        fault,
        HistoryFault::DeferredUnconfirmed | HistoryFault::DeferredPostCommit
    ));
    (fixture, parts)
}

#[test]
fn unconfirmed_deferred_history_keeps_owned_stage_and_stops_cleanup() {
    let (fixture, mut parts) = staged_fault(HistoryFault::DeferredUnconfirmed);
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::DeferredUnconfirmed);
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);
    let second = fixture.add_recording("z-next.m4a", &m4a(CAPTURED + 1, 1, b"next"));
    let error = Ingest {
        recorder: &probe,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default())
    .unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    assert!(error.known_receipt.is_none());
    assert_eq!(ledger.failed_calls.get(), 1);
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(ledger.base.seen(&second).unwrap().is_none());
    assert!(
        !ledger
            .base
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(
                record.fact,
                HistoryFact::WorkStep {
                    outcome: WorkOutcome::Deferred,
                    ..
                }
            ))
    );
}

#[test]
fn postcommit_deferred_history_keeps_receipt_and_stage_without_cleanup() {
    let (fixture, mut parts) = staged_fault(HistoryFault::DeferredPostCommit);
    let ledger = FaultLedger::new(parts.ledger, HistoryFault::DeferredPostCommit);
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);
    let error = Ingest {
        recorder: &probe,
        archive: &parts.archive,
        ledger: &ledger,
        clock: &parts.clock,
        trash: &parts.trash,
        sink: &mut parts.sink,
        settings: &parts.settings,
    }
    .run(&Mode::default())
    .unwrap_err();

    assert!(matches!(error.failure, IngestFailure::RequiredHistory(_)));
    let receipt = error
        .known_receipt
        .as_ref()
        .expect("known committed deferral");
    assert!(receipt.records.iter().any(|record| matches!(
        record.fact,
        HistoryFact::WorkStep {
            outcome: WorkOutcome::Deferred,
            ..
        }
    )));
    assert!(receipt.completed.is_empty());
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
}

#[test]
fn sink_stop_after_deferred_commit_keeps_stage_and_stops_following_work() {
    let fixture = Fixture::new();
    fixture.add_recording("a-stage.m4a", &m4a(CAPTURED, 1, b"whole source"));
    let second = fixture.add_recording("z-next.m4a", &m4a(CAPTURED + 1, 1, b"next"));
    let mut parts = fixture.parts();
    parts.sink.stop_at = Some(StopAt::Work);
    let probe = GateRecorder::new(&fixture.recordings);
    probe.clone_behavior.set(CloneBehavior::Truncated);

    let error = probed(&mut parts, &probe).unwrap_err();

    assert!(matches!(error.failure, IngestFailure::SinkStopped(_)));
    assert!(
        error
            .known_receipt
            .as_ref()
            .unwrap()
            .records
            .iter()
            .any(|record| matches!(
                record.fact,
                HistoryFact::WorkStep {
                    outcome: WorkOutcome::Deferred,
                    ..
                }
            ))
    );
    assert_eq!(parts.archive.staged_leftovers().unwrap().len(), 1);
    assert!(parts.trash.attempted.borrow().is_empty());
    assert!(parts.ledger.seen(&second).unwrap().is_none());
}
