use super::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use vpt_application::ports::{LedgerError, SinkError, SinkWarning};
use vpt_domain::history::{EventId, EventPathKey, RunId};
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportFailureClass, ExportPosition, HistoryRecord,
    HookFailureClass, RecoveryEvidence, RecoveryView,
};

pub(super) struct RefusingHistory(pub(super) LedgerError);

impl RunHistory for RefusingHistory {
    fn begin_run(&self, _: RunKind, _: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        unreachable!("required read refusal fixture")
    }

    fn begin_recovery(&self, _: UtcInstant, _: RecoveryEvidence) -> HistoryWrite<HistoryReceipt> {
        unreachable!("required read refusal fixture")
    }

    fn commit_step(
        &self,
        _: RunId,
        _: &WorkDelta,
        _: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        unreachable!("required read refusal fixture")
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        Err(self.0.clone())
    }

    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        Err(self.0.clone())
    }

    fn acknowledge_export(
        &self,
        _: EventId,
        _: Option<ExportCursor>,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        unreachable!("required read refusal fixture")
    }

    fn note_export_failure(&self, _: &ExportFailure) -> HistoryWrite<()> {
        unreachable!("required read refusal fixture")
    }

    fn restart_export(
        &self,
        _: ExportCursor,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        unreachable!("required read refusal fixture")
    }
}

#[test]
fn unsafe_state_or_required_history_failure_stops_new_work() {
    super::optional::assert_ack_failures_revalidate_before_hook();
    let fixture = Fixture::new();
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    fs::set_permissions(fixture.state.path(), fs::Permissions::from_mode(0o755)).unwrap();

    let outcome = sink(&fixture, &[], &[]).after_commit(&receipt);
    assert!(matches!(&outcome, SinkOutcome::Stop(stop) if stop.reason == SinkError::UnsafeState));
    assert_eq!(fixture.history.pending_exports().unwrap().len(), 2);
    assert!(!fixture.log_path().exists());

    for error in [
        LedgerError::Busy,
        LedgerError::PathEscape(fixture.state.path().into()),
    ] {
        fs::set_permissions(fixture.state.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let history = RefusingHistory(error.clone());
        let mut sink = LoggedEvents::new(&fixture.state, &history, &[], &[], || at(20));
        let outcome = sink.after_commit(&receipt);
        let expected = if matches!(&error, LedgerError::PathEscape(_)) {
            SinkError::UnsafeState
        } else {
            SinkError::RequiredHistory(error)
        };
        assert!(
            matches!(outcome, SinkOutcome::Stop(stop) if stop.reason == expected && stop.committed == Some(receipt.clone()))
        );
    }
}

#[test]
fn hook_status_failures_save_only_typed_content_free_facts() {
    for (mode, expected) in [
        ("exit7", HookFailureClass::Exited(7)),
        ("signal", HookFailureClass::Signaled(libc::SIGTERM)),
        ("absent", HookFailureClass::Spawn),
    ] {
        let fixture = Fixture::new();
        let script = fixture.path("status-hook");
        write_hook(&script);
        let capture = fixture.path("status-capture");
        let executable = if mode == "absent" {
            fixture.path("missing-hook")
        } else {
            script
        };
        let report = command(&executable, &capture, &fixture.log_path(), mode, &[]);
        let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);

        let outcome = sink(&fixture, &report, &[]).after_commit(&receipt);
        assert_eq!(
            outcome,
            SinkOutcome::Continue {
                warnings: BTreeSet::from([SinkWarning::HookFailed])
            },
            "{mode}"
        );
        let view = fixture.history.recovery_view().unwrap();
        let failures = view
            .records
            .iter()
            .filter_map(|record| match &record.fact {
                HistoryFact::HookFailed { event, failure } => Some((*event, *failure)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(failures, vec![(receipt.records[0].id, expected)], "{mode}");
        assert!(
            !fs::read_to_string(fixture.log_path())
                .unwrap()
                .contains("child-private-sentinel")
        );
    }
}

#[test]
fn encoding_failures_preserve_work_and_save_only_closed_failure_classes() {
    let fixture = Fixture::new();
    let script = fixture.path("encoding-hook");
    write_hook(&script);
    let capture = fixture.path("encoding-capture");
    let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
    let invalid = PathBuf::from(OsString::from_vec(vec![b'/', b'x', 0xff]));
    let event = Event {
        kind: EventKind::BriefWritten,
        state: EventState::Done,
        recording: None,
        detail: EventDetail::None,
        counts: EventCounts::default(),
        paths: vec![EventPath {
            key: EventPathKey::Brief,
            path: invalid,
        }],
        occurred_at: at(11),
    };
    let receipt = fixture.save_event(event);

    let outcome = sink(&fixture, &report, &[]).after_commit(&receipt);
    assert!(
        matches!(&outcome, SinkOutcome::Continue { warnings } if warnings.contains(&SinkWarning::ExportUnavailable) && warnings.contains(&SinkWarning::HookFailed))
    );
    assert!(!capture.with_extension("calls").exists());
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(view.records[1].id, receipt.records[0].id);
    assert!(
        view.failures
            .iter()
            .all(|saved| saved.failure.class == ExportFailureClass::Encoding)
    );
    assert!(view.records.iter().any(|record| matches!(&record.fact, HistoryFact::HookFailed { event, failure: HookFailureClass::Encoding } if *event == receipt.records[0].id)));
    assert_eq!(fixture.documents()[0]["fact"]["tag"], "run_started");
    assert!(
        fixture
            .history
            .pending_exports()
            .unwrap()
            .iter()
            .any(|record| record.id == receipt.records[0].id)
    );
}
