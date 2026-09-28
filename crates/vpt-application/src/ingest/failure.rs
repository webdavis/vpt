use super::{Ingest, IngestError, IngestFailure, IngestReport};
use crate::ports::{
    Archive, Clock, EventSink, HistoryReceipt, HistoryWrite, RecorderStore, RecordingLedger,
    RunHistory, SinkOutcome, Trash, WorkDelta,
};
use std::path::PathBuf;
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
    pub(super) fn required(
        write: HistoryWrite<HistoryReceipt>,
        report: &IngestReport,
    ) -> Result<HistoryReceipt, Box<IngestError>> {
        match write {
            HistoryWrite::Committed(receipt) => Ok(receipt),
            HistoryWrite::CommittedWithFailure { receipt, error } => Err(Self::error(
                report,
                IngestFailure::RequiredHistory(error),
                Some(receipt),
            )),
            HistoryWrite::Unconfirmed(error) => Err(Self::error(
                report,
                IngestFailure::RequiredHistory(error),
                None,
            )),
        }
    }

    pub(super) fn export(
        &mut self,
        receipt: &HistoryReceipt,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        match self.sink.after_commit(receipt) {
            SinkOutcome::Continue { warnings } => {
                report.warnings.extend(warnings);
                Ok(())
            }
            SinkOutcome::Stop(stop) => {
                report.warnings.extend(stop.warnings);
                let mut error = Self::error(
                    report,
                    IngestFailure::SinkStopped(stop.reason),
                    stop.committed,
                );
                for id in &receipt.completed {
                    if !error.completed.contains(id) {
                        error.completed.push(id.clone());
                    }
                }
                Err(error)
            }
        }
    }

    pub(super) fn error(
        report: &IngestReport,
        failure: IngestFailure,
        known_receipt: Option<HistoryReceipt>,
    ) -> Box<IngestError> {
        let mut completed = report.recovered.clone();
        for record in &report.ingested {
            if !completed.contains(&record.id) {
                completed.push(record.id.clone());
            }
        }
        if let Some(receipt) = &known_receipt {
            for id in &receipt.completed {
                if !completed.contains(id) {
                    completed.push(id.clone());
                }
            }
        }
        Box::new(IngestError {
            failure,
            completed,
            known_receipt,
            warnings: report.warnings.clone(),
            log: report.log.clone(),
            owned_staging: None,
        })
    }

    pub(super) fn owned_error(
        report: &IngestReport,
        failure: IngestFailure,
        owned_staging: PathBuf,
    ) -> Box<IngestError> {
        let mut error = Self::error(report, failure, None);
        error.owned_staging = Some(owned_staging);
        error
    }

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
            self.discard_owned(&staged, report);
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
        | IngestFailure::Recorder(_)
        | IngestFailure::DatalessUnavailable(_) => SafeFailureClass::SourceUnavailable,
        IngestFailure::NoSpace
        | IngestFailure::Sync(_)
        | IngestFailure::ArchiveCollision { .. }
        | IngestFailure::Archive(_)
        | IngestFailure::HelperVersion { .. } => SafeFailureClass::ArchiveFailed,
        IngestFailure::Ledger(_) | IngestFailure::RequiredHistory(_) => {
            SafeFailureClass::LedgerUnavailable
        }
        IngestFailure::SinkStopped(_) => SafeFailureClass::ExportUnavailable,
        IngestFailure::DryRunUnavailable | IngestFailure::PathEscape(_) => {
            SafeFailureClass::ConfigurationRefused
        }
    }
}
