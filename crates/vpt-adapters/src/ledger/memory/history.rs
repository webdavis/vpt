use super::{MemoryLedger, State, with};
use vpt_application::ports::{HistoryReceipt, HistoryWrite, LedgerError, RunHistory, WorkDelta};
use vpt_domain::history::{EventId, ExportFailureId, RunId};
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, ExportRestart, HistoryFact, HistoryRecord,
    RecoveryEvidence, RecoveryView, RunKind, SavedExportFailure,
};
use vpt_domain::time::UtcInstant;

mod releases;
mod retention;
mod state;
mod step;
pub(super) use state::HistoryState;
use state::{NAMESPACE, conflict};

impl MemoryLedger {
    fn history_write<T>(
        &self,
        operation: impl FnOnce(&mut State) -> Result<T, LedgerError>,
    ) -> HistoryWrite<T> {
        match with(self, |state| {
            let mut next = state.clone();
            let receipt = operation(&mut next)?;
            *state = next;
            Ok(receipt)
        }) {
            Ok(Ok(receipt)) => HistoryWrite::Committed(receipt),
            Ok(Err(error)) | Err(error) => HistoryWrite::Unconfirmed(error),
        }
    }
}

impl RunHistory for MemoryLedger {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|state| {
            state.history.next_run = state.history.next_run.checked_add(1).ok_or_else(conflict)?;
            let run = RunId::new(NAMESPACE, state.history.next_run);
            let record = state
                .history
                .append(run, at, HistoryFact::RunStarted(kind))?;
            Ok(HistoryReceipt {
                run,
                records: vec![record],
                completed: vec![],
            })
        })
    }

    fn begin_recovery(
        &self,
        at: UtcInstant,
        evidence: RecoveryEvidence,
    ) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|state| state.history.begin_recovery(at, evidence))
    }

    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|state| step::apply(state, run, delta, facts))
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        with(self, |state| RecoveryView {
            records: state.history.records.clone(),
            failures: state.history.failures.clone(),
            restarts: state.history.restarts.clone(),
            active_cursor: state.history.cursor,
            summarized_through: state.history.summarized_through(),
        })
    }

    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        with(self, |state| {
            state
                .history
                .records
                .iter()
                .filter(|record| record.appended_at.is_none())
                .cloned()
                .collect()
        })
    }

    fn acknowledge_export(
        &self,
        event: EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.history_write(|state| {
            let history = &mut state.history;
            if history.cursor != expected || event.namespace() != NAMESPACE {
                return Err(conflict());
            }
            if let Some(cursor) = expected {
                if position.device != cursor.position.device
                    || position.inode != cursor.position.inode
                    || position.end_offset <= cursor.position.end_offset
                {
                    return Err(conflict());
                }
            } else if position.end_offset == 0 {
                return Err(conflict());
            }
            let first = history
                .records
                .iter_mut()
                .find(|record| record.appended_at.is_none())
                .ok_or_else(conflict)?;
            if first.id != event {
                return Err(conflict());
            }
            first.appended_at = Some(appended_at);
            first.export_position = Some(position);
            history.cursor = Some(ExportCursor {
                generation: expected.map_or(0, |cursor| cursor.generation),
                position,
            });
            Ok(())
        })
    }

    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()> {
        self.history_write(|state| {
            let history = &mut state.history;
            if failure.event.namespace() != NAMESPACE
                || !history
                    .records
                    .iter()
                    .any(|record| record.id == failure.event && record.appended_at.is_none())
                || failure.last_confirmed
                    != history
                        .records
                        .iter()
                        .rev()
                        .find_map(|record| record.export_position)
            {
                return Err(conflict());
            }
            history.next_failure = history.next_failure.checked_add(1).ok_or_else(conflict)?;
            history.failures.push(SavedExportFailure {
                id: ExportFailureId::new(NAMESPACE, history.next_failure),
                failure: failure.clone(),
            });
            Ok(())
        })
    }

    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.history_write(|state| {
            let history = &mut state.history;
            if history.cursor != Some(expected) {
                if history
                    .restarts
                    .iter()
                    .any(|restart| restart.old == expected && restart.new.position == observed)
                {
                    return Ok(());
                }
                return Err(conflict());
            }
            if observed.device == expected.position.device
                && observed.inode == expected.position.inode
                && observed.end_offset >= expected.position.end_offset
            {
                return Err(conflict());
            }
            let generation = expected.generation.checked_add(1).ok_or_else(conflict)?;
            let new = ExportCursor {
                generation,
                position: observed,
            };
            history.restarts.push(ExportRestart {
                old: expected,
                new,
                observed_at,
            });
            for record in &mut history.records {
                record.appended_at = None;
                record.export_position = None;
            }
            history.cursor = Some(new);
            Ok(())
        })
    }
}
