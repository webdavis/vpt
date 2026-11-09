use super::*;
use vpt_domain::history::{EventId, ExportFailureId, LedgerNamespace};
use vpt_domain::run::{ExportFailureClass, SavedExportFailure};

#[derive(Default)]
struct RecordingSink {
    delivered: Vec<HistoryReceipt>,
    recovery: Vec<RecoveryView>,
    next: Option<SinkOutcome>,
}

impl EventSink for RecordingSink {
    fn after_commit(&mut self, committed: &HistoryReceipt) -> SinkOutcome {
        self.delivered.push(committed.clone());
        self.next.take().unwrap_or_else(|| SinkOutcome::Continue {
            warnings: BTreeSet::new(),
        })
    }

    fn recover(&mut self, view: &RecoveryView) -> SinkOutcome {
        self.recovery.push(view.clone());
        self.next.take().unwrap_or_else(|| SinkOutcome::Continue {
            warnings: BTreeSet::new(),
        })
    }
}

fn receipt() -> HistoryReceipt {
    let namespace = LedgerNamespace::new([9; 16]);
    let run = RunId::new(namespace, 1);
    HistoryReceipt {
        run,
        records: vec![HistoryRecord {
            id: EventId::new(namespace, 2),
            run,
            occurred_at: UtcInstant { secs: 11 },
            fact: HistoryFact::RunStarted(RunKind::Ingest),
            appended_at: None,
            export_position: None,
        }],
        completed: vec![RecordingId::parse("2026-08-24T144736-4f3ab19c02de").unwrap()],
    }
}

#[test]
fn recording_sink_keeps_recovery_evidence_separate_from_event_delivery() {
    let mut sink = RecordingSink::default();
    let committed = receipt();
    let failure_id = ExportFailureId::new(committed.run.namespace(), 1);
    let view = RecoveryView {
        records: committed.records.clone(),
        failures: vec![SavedExportFailure {
            id: failure_id,
            failure: ExportFailure {
                event: committed.records[0].id,
                observed_at: UtcInstant { secs: 12 },
                class: ExportFailureClass::Io,
                last_confirmed: None,
                observed_file: None,
                partial: None,
            },
        }],
        ..Default::default()
    };
    let empty = SinkOutcome::Continue {
        warnings: BTreeSet::new(),
    };

    assert_eq!(sink.after_commit(&committed), empty);
    assert_eq!(sink.delivered, vec![committed.clone()]);
    assert!(sink.recovery.is_empty());
    assert_eq!(sink.recover(&view), empty);
    assert_eq!(sink.recovery, vec![view]);
    assert_eq!(sink.delivered, vec![committed]);
}

#[test]
fn recording_sink_preserves_typed_stop_receipt_and_warning_set() {
    let mut sink = RecordingSink::default();
    let committed = receipt();
    let warnings = BTreeSet::from([SinkWarning::ExportUnavailable, SinkWarning::HookFailed]);
    let stop = SinkOutcome::Stop(Box::new(SinkStop {
        reason: SinkError::RequiredHistory(LedgerError::Busy),
        committed: Some(committed.clone()),
        warnings,
    }));
    sink.next = Some(stop.clone());

    assert_eq!(sink.after_commit(&committed), stop);
    assert_eq!(sink.delivered, vec![committed]);
}
