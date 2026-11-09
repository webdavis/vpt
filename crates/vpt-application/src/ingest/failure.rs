use super::{Ingest, IngestError, IngestFailure, IngestReport};
use crate::ports::{
    Archive, ArchiveError, Clock, EventSink, RecorderStore, RecordingLedger, RunHistory, Trash,
    WorkDelta,
};
use vpt_domain::history::{
    Event, EventCounts, EventDetail, EventKind, EventState, RunId, SafeFailureClass,
};
use vpt_domain::run::{HistoryFact, RunOutcome};
use vpt_domain::time::UtcInstant;

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub(super) fn finish_failure(
        &mut self,
        run: RunId,
        occurred_at: UtcInstant,
        mut error: Box<IngestError>,
        report: &mut IngestReport,
    ) -> Box<IngestError> {
        if matches!(
            &error.failure,
            IngestFailure::RequiredHistory(_)
                | IngestFailure::SinkStopped(_)
                | IngestFailure::Ledger(_)
        ) {
            return error;
        }
        let preserve_owned = matches!(&error.failure, IngestFailure::PathEscape(_));
        let event = Event {
            kind: EventKind::IngestFailed,
            state: EventState::Failed,
            occasion: None,
            recording: None,
            detail: EventDetail::Failure(safe_failure_class(&error.failure)),
            counts: EventCounts::default(),
            paths: Vec::new(),
            occurred_at,
        };
        let facts = [
            (occurred_at, HistoryFact::Event(event)),
            (occurred_at, HistoryFact::RunFinished(RunOutcome::Failed)),
        ];
        let receipt = match Self::required(
            self.ledger.commit_step(run, &WorkDelta::default(), &facts),
            report,
        ) {
            Ok(receipt) => receipt,
            Err(required) => return required,
        };
        if let Err(stopped) = self.export(&receipt, report) {
            return stopped;
        }
        if !preserve_owned && let Some(staged) = error.owned_staging.take() {
            let _ = self.discard_owned(&staged, report);
        }
        error.warnings = report.warnings.clone();
        error.log = report.log.clone();
        error
    }
}

fn safe_failure_class(failure: &IngestFailure) -> SafeFailureClass {
    match failure {
        IngestFailure::StoreUnreadable(_)
        | IngestFailure::EmptyStore
        | IngestFailure::OnceMissing(_)
        | IngestFailure::Recorder(_) => SafeFailureClass::SourceUnavailable,
        IngestFailure::NoSpace
        | IngestFailure::Sync(_)
        | IngestFailure::ArchiveCollision { .. }
        | IngestFailure::Archive(_)
        | IngestFailure::HelperVersion { .. } => SafeFailureClass::ArchiveFailed,
        IngestFailure::Ledger(_) | IngestFailure::RequiredHistory(_) => {
            SafeFailureClass::LedgerUnavailable
        }
        IngestFailure::SinkStopped(_) => SafeFailureClass::ExportUnavailable,
        IngestFailure::PathEscape(_) => SafeFailureClass::ConfigurationRefused,
    }
}

pub(super) fn archive_failure(error: ArchiveError) -> IngestFailure {
    match error {
        ArchiveError::NoSpace => IngestFailure::NoSpace,
        ArchiveError::Changed(_) => {
            IngestFailure::Sync("archive changed during publication".into())
        }
        ArchiveError::Sync(detail) => IngestFailure::Sync(detail),
        ArchiveError::PlacedUnsynced(path) => IngestFailure::Sync(format!(
            "archive placed without directory sync at {}",
            path.display()
        )),
        ArchiveError::Escape(path) => IngestFailure::PathEscape(path),
        ArchiveError::Io(detail) => IngestFailure::Archive(detail),
    }
}
