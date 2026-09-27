use super::{LedgerCommit, LedgerError};
use std::path::PathBuf;
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, HistoryRecord, RecoveryView, RunKind,
};
use vpt_domain::time::UtcInstant;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkDelta {
    pub commit: LedgerCommit,
    pub source_updates: Vec<(RecordingId, PathBuf)>,
    pub publication_clears: Vec<PathBuf>,
    pub audio_trash_marks: Vec<(RecordingId, UtcInstant)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryReceipt {
    pub run: RunId,
    pub records: Vec<HistoryRecord>,
    pub completed: Vec<RecordingId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryWrite<T> {
    Committed(T),
    CommittedWithFailure { receipt: T, error: LedgerError },
    Unconfirmed(LedgerError),
}

pub trait RunHistory {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt>;
    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt>;
    fn recovery_view(&self) -> Result<RecoveryView, LedgerError>;
    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError>;
    fn acknowledge_export(
        &self,
        event: vpt_domain::history::EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()>;
    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()>;
    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkOutcome {
    Delivered,
    Warning(ExportFailure),
}

pub trait EventSink {
    fn after_commit(&self, record: &HistoryRecord) -> SinkOutcome;
    fn recover(&self, view: &RecoveryView) -> SinkOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        delivered: Mutex<Vec<HistoryRecord>>,
        recovery: Mutex<Vec<RecoveryView>>,
    }

    impl EventSink for RecordingSink {
        fn after_commit(&self, record: &HistoryRecord) -> SinkOutcome {
            self.delivered.lock().unwrap().push(record.clone());
            SinkOutcome::Delivered
        }

        fn recover(&self, view: &RecoveryView) -> SinkOutcome {
            self.recovery.lock().unwrap().push(view.clone());
            SinkOutcome::Delivered
        }
    }

    #[test]
    fn recording_sink_keeps_recovery_evidence_separate_from_event_delivery() {
        let sink = RecordingSink::default();
        let view = RecoveryView::default();
        assert_eq!(sink.recover(&view), SinkOutcome::Delivered);
        assert_eq!(sink.recovery.lock().unwrap().as_slice(), &[view]);
        assert!(sink.delivered.lock().unwrap().is_empty());
    }
}
