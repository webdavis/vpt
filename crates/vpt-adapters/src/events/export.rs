use super::{LoggedEvents, required, stop};
use crate::{AppendFailure, AppendReceipt, EventLog, LogInspection};
use std::collections::BTreeSet;
use vpt_application::ports::{HistoryReceipt, RunHistory, SinkError, SinkStop, SinkWarning};
use vpt_domain::run::{ExportCursor, ExportFailureClass, ExportPosition};
use vpt_protocol::encode_history;

mod failure;
use failure::{FailureObservation, partial};

impl<H: RunHistory, N: FnMut() -> vpt_domain::time::UtcInstant> LoggedEvents<'_, H, N> {
    pub(super) fn export_pending_with_append(
        &mut self,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
        mut append: impl FnMut(&mut EventLog, &str) -> Result<AppendReceipt, Box<AppendFailure>>,
    ) -> Result<(), Box<SinkStop>> {
        self.state
            .validate_private_state()
            .map_err(|_| stop(SinkError::UnsafeState, committed, warnings))?;
        let mut view = self
            .history
            .recovery_view()
            .map_err(|error| required(error, committed, warnings))?;
        let mut pending = self
            .history
            .pending_exports()
            .map_err(|error| required(error, committed, warnings))?;
        if pending.is_empty() && view.active_cursor.is_none() {
            return Ok(());
        }
        let mut log = match self.state.open_event_log() {
            Ok(log) => log,
            Err(error) => {
                if pending.is_empty() {
                    return self.unavailable_without_pending(error, committed, warnings);
                }
                return self.log_failure(
                    pending[0].id,
                    error,
                    FailureObservation {
                        inspection: None,
                        partial: None,
                        last_confirmed: view
                            .records
                            .iter()
                            .rev()
                            .find_map(|record| record.export_position),
                    },
                    committed,
                    warnings,
                );
            }
        };
        let mut inspection = match log.inspect() {
            Ok(inspection) => inspection,
            Err(error) => {
                if pending.is_empty() {
                    return self.unavailable_without_pending(error, committed, warnings);
                }
                return self.log_failure(
                    pending[0].id,
                    error,
                    FailureObservation {
                        inspection: None,
                        partial: None,
                        last_confirmed: view
                            .records
                            .iter()
                            .rev()
                            .find_map(|record| record.export_position),
                    },
                    committed,
                    warnings,
                );
            }
        };
        if let Some(expected) = view.active_cursor {
            let observed = ExportPosition {
                device: inspection.device,
                inode: inspection.inode,
                end_offset: inspection.complete_end_offset,
            };
            let changed_file = observed.device != expected.position.device
                || observed.inode != expected.position.inode;
            let shortened = observed.end_offset < expected.position.end_offset;
            if changed_file || shortened {
                let at = (self.clock)();
                let confirmed = self.optional(
                    self.history.restart_export(expected, observed, at),
                    SinkWarning::ExportProgressUnconfirmed,
                    committed,
                    warnings,
                )?;
                if !confirmed {
                    return Ok(());
                }
                view = self
                    .history
                    .recovery_view()
                    .map_err(|error| required(error, committed, warnings))?;
                pending = self
                    .history
                    .pending_exports()
                    .map_err(|error| required(error, committed, warnings))?;
            }
        }
        let mut last_confirmed = view
            .records
            .iter()
            .rev()
            .find_map(|record| record.export_position);
        let mut cursor = view.active_cursor;
        for record in pending {
            let at = (self.clock)();
            let line = match encode_history(&record, at) {
                Ok(line) => line,
                Err(_) => {
                    warnings.insert(SinkWarning::ExportUnavailable);
                    self.note_failure(
                        record.id,
                        ExportFailureClass::Encoding,
                        FailureObservation {
                            inspection: Some(inspection),
                            partial: None,
                            last_confirmed,
                        },
                        committed,
                        warnings,
                    )?;
                    return Ok(());
                }
            };
            let receipt = match append(&mut log, &line) {
                Ok(receipt) => receipt,
                Err(failure) => {
                    let observed = failure.effects.inspection;
                    let partial = partial(failure.effects);
                    return self.log_failure(
                        record.id,
                        failure.error,
                        FailureObservation {
                            inspection: observed,
                            partial,
                            last_confirmed,
                        },
                        committed,
                        warnings,
                    );
                }
            };
            let acknowledged = ExportPosition {
                device: receipt.device,
                inode: receipt.inode,
                end_offset: receipt.end_offset,
            };
            let confirmed = self.optional(
                self.history
                    .acknowledge_export(record.id, cursor, acknowledged, at),
                SinkWarning::ExportProgressUnconfirmed,
                committed,
                warnings,
            )?;
            if !confirmed {
                return Ok(());
            }
            cursor = Some(ExportCursor {
                generation: cursor.map_or(0, |cursor| cursor.generation),
                position: acknowledged,
            });
            last_confirmed = Some(acknowledged);
            inspection = LogInspection {
                device: receipt.device,
                inode: receipt.inode,
                complete_end_offset: receipt.end_offset,
                physical_end_offset: receipt.end_offset,
            };
        }
        Ok(())
    }

    pub(super) fn export_pending(
        &mut self,
        committed: Option<&HistoryReceipt>,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<(), Box<SinkStop>> {
        self.export_pending_with_append(committed, warnings, EventLog::append_line)
    }
}
