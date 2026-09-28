use super::{SqliteLedger, connection, error, map};
use crate::database::AccessPolicy;
use rusqlite::{Transaction, TransactionBehavior};
mod codec;
mod export;
mod storage;
mod store;
use vpt_application::ports::{HistoryReceipt, HistoryWrite, LedgerError, RunHistory, WorkDelta};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, HistoryRecord, RecoveryEvidence,
    RecoveryView, RunKind,
};
use vpt_domain::time::UtcInstant;

impl SqliteLedger {
    fn history_write<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, LedgerError>,
    ) -> HistoryWrite<T> {
        self.history_write_with(operation, || {}, |closed| closed)
    }

    fn history_write_with<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, LedgerError>,
        after_commit: impl FnOnce(),
        close_result: impl FnOnce(Result<(), LedgerError>) -> Result<(), LedgerError>,
    ) -> HistoryWrite<T> {
        if self.policy != AccessPolicy::WritableLedger {
            return HistoryWrite::Unconfirmed(LedgerError::Io("read-only ledger".into()));
        }
        let mut owner = match connection::open(&self.state, self.busy, self.policy) {
            Ok(owner) => owner,
            Err(error) => return HistoryWrite::Unconfirmed(error),
        };
        let mut committed = None;
        let result = owner.with_connection(|connection| {
            let transaction =
                Transaction::new_unchecked(connection, TransactionBehavior::Immediate)
                    .map_err(map)?;
            let receipt = operation(&transaction)?;
            self.state.revalidate().map_err(error::contained)?;
            transaction.commit().map_err(map)?;
            committed = Some(receipt);
            after_commit();
            Ok(())
        });
        let closed = close_result(owner.try_close().map_err(Into::into));
        let finished = error::finish(result, closed);
        match (finished, committed) {
            (Ok(()), Some(receipt)) => HistoryWrite::Committed(receipt),
            (Err(error), Some(receipt)) => HistoryWrite::CommittedWithFailure { receipt, error },
            (Err(error), None) => HistoryWrite::Unconfirmed(error),
            (Ok(()), None) => {
                HistoryWrite::Unconfirmed(LedgerError::Corrupt("missing history receipt".into()))
            }
        }
    }
}

impl RunHistory for SqliteLedger {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        store::begin(self, kind, at)
    }

    fn begin_recovery(
        &self,
        at: UtcInstant,
        evidence: RecoveryEvidence,
    ) -> HistoryWrite<HistoryReceipt> {
        store::begin_recovery(self, at, evidence)
    }

    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        store::step(self, run, delta, facts)
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        export::view(self)
    }

    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        self.read(|connection| storage::records(connection, true))
    }

    fn acknowledge_export(
        &self,
        event: EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()> {
        export::acknowledge(self, event, expected, position, appended_at)
    }

    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()> {
        export::note_failure(self, failure)
    }

    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()> {
        export::restart(self, expected, observed, observed_at)
    }
}

#[cfg(test)]
mod tests;
