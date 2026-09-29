use super::*;
use std::cell::Cell;
use vpt_application::ports::{LedgerError, SinkError, SinkWarning};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{ExportCursor, ExportFailure, ExportPosition, HistoryRecord, RecoveryView};

#[derive(Clone, Copy, Debug)]
pub(super) enum AckFault {
    Unconfirmed,
    KnownPostCommit,
    DefiniteEscape,
}

pub(super) struct FaultHistory<'a> {
    pub(super) inner: &'a MemoryLedger,
    pub(super) state_path: PathBuf,
    pub(super) fault: AckFault,
    pub(super) invalidate_state: bool,
    pub(super) ack_attempts: Cell<u32>,
}

impl RunHistory for FaultHistory<'_> {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        self.inner.begin_run(kind, at)
    }

    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        self.inner.commit_step(run, delta, facts)
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        self.inner.recovery_view()
    }

    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        self.inner.pending_exports()
    }

    fn acknowledge_export(
        &self,
        event: EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.ack_attempts.set(self.ack_attempts.get() + 1);
        let committed = if matches!(self.fault, AckFault::KnownPostCommit) {
            Some(
                self.inner
                    .acknowledge_export(event, expected, position, appended_at),
            )
        } else {
            None
        };
        if self.invalidate_state {
            fs::set_permissions(&self.state_path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        match self.fault {
            AckFault::Unconfirmed => HistoryWrite::Unconfirmed(LedgerError::Busy),
            AckFault::KnownPostCommit => {
                assert!(matches!(committed, Some(HistoryWrite::Committed(()))));
                HistoryWrite::CommittedWithFailure {
                    receipt: (),
                    error: LedgerError::Io("private-root-secret-canary".into()),
                }
            }
            AckFault::DefiniteEscape => {
                HistoryWrite::Unconfirmed(LedgerError::PathEscape(self.state_path.clone()))
            }
        }
    }

    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()> {
        self.inner.note_export_failure(failure)
    }

    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.inner.restart_export(expected, observed, at)
    }
}

pub(super) fn assert_ack_failures_revalidate_before_hook() {
    for (fault, invalidate_state) in [
        (AckFault::Unconfirmed, false),
        (AckFault::Unconfirmed, true),
        (AckFault::KnownPostCommit, false),
        (AckFault::KnownPostCommit, true),
        (AckFault::DefiniteEscape, false),
    ] {
        let fixture = Fixture::new();
        let script = fixture.path("optional-hook");
        write_hook(&script);
        let capture = fixture.path("optional-capture");
        let report = command(&script, &capture, &fixture.log_path(), "ok", &[]);
        let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
        let history = FaultHistory {
            inner: &fixture.history,
            state_path: fixture.state.path().into(),
            fault,
            invalidate_state,
            ack_attempts: Cell::new(0),
        };
        let mut sink = LoggedEvents::new(&fixture.state, &history, &report, &[], || at(20));
        let outcome = sink.after_commit(&receipt);
        let unsafe_state = invalidate_state || matches!(fault, AckFault::DefiniteEscape);
        if unsafe_state {
            assert!(
                matches!(&outcome, SinkOutcome::Stop(stop) if stop.reason == SinkError::UnsafeState && stop.committed == Some(receipt.clone())),
                "{fault:?}"
            );
            assert!(
                !capture.with_extension("calls").exists(),
                "unsafe state reached hook"
            );
        } else {
            let expected = match fault {
                AckFault::Unconfirmed => SinkWarning::ExportProgressUnconfirmed,
                AckFault::KnownPostCommit => SinkWarning::ExportBookkeepingPostCommitFailure,
                AckFault::DefiniteEscape => unreachable!(),
            };
            assert!(
                matches!(&outcome, SinkOutcome::Continue { warnings } if warnings.contains(&expected)),
                "{fault:?}"
            );
            assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
        }
        assert!(!format!("{outcome:?}").contains("private-root-secret-canary"));
        let view = fixture.history.recovery_view().unwrap();
        assert_eq!(view.records[1].id, receipt.records[0].id);
        if matches!(fault, AckFault::KnownPostCommit) {
            assert!(view.records[0].export_position.is_some());
        } else {
            assert!(view.records[0].export_position.is_none());
        }
    }
}

#[test]
fn unconfirmed_ack_then_failed_hook_does_not_retry_export_inline() {
    let fixture = Fixture::new();
    let script = fixture.path("ack-outage-hook");
    write_hook(&script);
    let capture = fixture.path("ack-outage-capture");
    let report = command(&script, &capture, &fixture.log_path(), "exit7", &[]);
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    let history = FaultHistory {
        inner: &fixture.history,
        state_path: fixture.state.path().into(),
        fault: AckFault::Unconfirmed,
        invalidate_state: false,
        ack_attempts: Cell::new(0),
    };
    let mut sink = LoggedEvents::new(&fixture.state, &history, &report, &[], || at(20));

    let outcome = sink.after_commit(&receipt);
    assert_eq!(
        outcome,
        SinkOutcome::Continue {
            warnings: BTreeSet::from([
                SinkWarning::ExportProgressUnconfirmed,
                SinkWarning::HookFailed
            ])
        }
    );
    assert_eq!(
        history.ack_attempts.get(),
        1,
        "unconfirmed append was retried inline"
    );
    assert_eq!(
        fixture.documents().len(),
        1,
        "same record was appended again"
    );
    assert_eq!(fs::read(capture.with_extension("calls")).unwrap(), b"x");
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(view.records.len(), 3);
    assert!(
        matches!(&view.records[2].fact, HistoryFact::HookFailed { event, .. } if *event == receipt.records[0].id)
    );
    assert_eq!(fixture.history.pending_exports().unwrap().len(), 3);
}
