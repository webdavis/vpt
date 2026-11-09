use super::{Ingest, IngestError, IngestFailure, IngestReport};
use crate::ports::{
    Archive, Clock, EventSink, HistoryReceipt, HistoryWrite, RecorderStore, RecordingLedger,
    RunHistory, SinkOutcome, Trash,
};
use std::path::PathBuf;

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
        for id in &report.already_ingested {
            if !completed.contains(id) {
                completed.push(id.clone());
            }
        }
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
}
