use super::super::{LoggedEvents, stop};
use crate::{AppendEffects, AppendSync, LogError, LogInspection};
use std::collections::BTreeSet;
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, LedgerError, RunHistory, SinkError, SinkStop, SinkWarning,
};
use vpt_domain::history::EventId;
use vpt_domain::run::{
    ExportFailure, ExportFailureClass, ExportPosition, ObservedLogFile, PartialAppend,
};
use vpt_domain::time::UtcInstant;

pub(super) struct FailureObservation {
    pub(super) inspection: Option<LogInspection>,
    pub(super) partial: Option<PartialAppend>,
    pub(super) last_confirmed: Option<ExportPosition>,
}

impl<H: RunHistory, N: FnMut() -> UtcInstant> LoggedEvents<'_, H, N> {
    pub(super) fn unavailable_without_pending(
        &self,
        error: LogError,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<(), Box<SinkStop>> {
        if matches!(error, LogError::State(_)) {
            return Err(stop(SinkError::UnsafeState, committed, warnings));
        }
        self.state
            .validate_private_state()
            .map_err(|_| stop(SinkError::UnsafeState, committed, warnings))?;
        warnings.insert(SinkWarning::ExportUnavailable);
        Ok(())
    }

    pub(super) fn log_failure(
        &mut self,
        event: EventId,
        error: LogError,
        observation: FailureObservation,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<(), Box<SinkStop>> {
        if matches!(&error, LogError::State(_)) {
            return Err(stop(SinkError::UnsafeState, committed, warnings));
        }
        self.state
            .validate_private_state()
            .map_err(|_| stop(SinkError::UnsafeState, committed, warnings))?;
        let class = match error {
            LogError::Unsafe(_) => ExportFailureClass::LeafRefused,
            LogError::InvalidLine => ExportFailureClass::Encoding,
            LogError::Io(_) | LogError::LengthChanged => ExportFailureClass::Io,
            LogError::State(_) => unreachable!(),
        };
        warnings.insert(SinkWarning::ExportUnavailable);
        self.note_failure(event, class, observation, committed, warnings)
    }

    pub(super) fn note_failure(
        &mut self,
        event: EventId,
        class: ExportFailureClass,
        observation: FailureObservation,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<(), Box<SinkStop>> {
        let failure = ExportFailure {
            event,
            observed_at: (self.clock)(),
            class,
            last_confirmed: observation.last_confirmed,
            observed_file: observation.inspection.map(observed),
            partial: observation.partial,
        };
        self.optional(
            self.history.note_export_failure(&failure),
            SinkWarning::ExportFailureEvidenceUnconfirmed,
            committed,
            warnings,
        )?;
        Ok(())
    }

    pub(super) fn optional(
        &self,
        write: HistoryWrite<()>,
        unconfirmed: SinkWarning,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<bool, Box<SinkStop>> {
        match write {
            HistoryWrite::Committed(()) => Ok(true),
            HistoryWrite::CommittedWithFailure { error, .. } => {
                self.optional_error(
                    error,
                    SinkWarning::ExportBookkeepingPostCommitFailure,
                    committed,
                    warnings,
                )?;
                Ok(true)
            }
            HistoryWrite::Unconfirmed(error) => {
                self.optional_error(error, unconfirmed, committed, warnings)?;
                Ok(false)
            }
        }
    }

    fn optional_error(
        &self,
        error: LedgerError,
        warning: SinkWarning,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<(), Box<SinkStop>> {
        if matches!(&error, LedgerError::PathEscape(_)) {
            return Err(stop(SinkError::UnsafeState, committed, warnings));
        }
        warnings.insert(warning);
        self.state
            .validate_private_state()
            .map_err(|_| stop(SinkError::UnsafeState, committed, warnings))
    }
}

fn observed(inspection: LogInspection) -> ObservedLogFile {
    ObservedLogFile {
        device: inspection.device,
        inode: inspection.inode,
        complete_end_offset: inspection.complete_end_offset,
        physical_end_offset: inspection.physical_end_offset,
    }
}

pub(super) fn partial(effects: AppendEffects) -> Option<PartialAppend> {
    let inspection = effects.inspection?;
    let repair = effects.repair;
    Some(PartialAppend {
        original_end: repair.map_or(inspection.physical_end_offset, |repair| repair.original_end),
        retained_end: repair.map_or(inspection.complete_end_offset, |repair| repair.retained_end),
        tail_repair_synced: repair.is_some_and(|repair| repair.synced),
        bytes_written: effects.written_bytes,
        file_synced: matches!(
            effects.synced,
            AppendSync::File | AppendSync::FileAndDirectory
        ),
        directory_synced: matches!(effects.synced, AppendSync::FileAndDirectory),
        identity_revalidated: false,
    })
}
