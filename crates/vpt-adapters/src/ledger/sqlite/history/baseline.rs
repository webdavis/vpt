use super::{SqliteLedger, map, storage};
use rusqlite::params;
use vpt_application::ports::{HistoryReceipt, HistoryWrite, LedgerError};
use vpt_domain::history::RunId;
use vpt_domain::run::{HistoryFact, RecoveryBaseline, RecoveryEvidence, RunKind};
use vpt_domain::time::UtcInstant;

pub(super) fn snapshot_baseline(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<RecoveryBaseline, LedgerError> {
    let namespace = storage::namespace(transaction)?;
    let (pending_from, through): (Option<i64>, Option<i64>) = transaction
        .query_row(
            "SELECT MIN(CASE WHEN appended_at IS NULL THEN sequence END), MAX(sequence) FROM history_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(storage::column)?;
    let baseline = RecoveryBaseline {
        generation: storage::cursor(transaction)?.map(|cursor| cursor.generation),
        pending_from: pending_from
            .map(storage::positive)
            .transpose()?
            .map(|sequence| vpt_domain::history::EventId::new(namespace, sequence)),
        through: through
            .map(storage::positive)
            .transpose()?
            .map(|sequence| vpt_domain::history::EventId::new(namespace, sequence)),
    };
    baseline
        .is_valid_for(namespace)
        .then_some(baseline)
        .ok_or_else(storage::conflict)
}

pub(super) fn expected_baseline(
    transaction: &rusqlite::Transaction<'_>,
    run: RunId,
) -> Result<RecoveryBaseline, LedgerError> {
    for record in storage::records(transaction, false)? {
        if record.run == run
            && let HistoryFact::RecoveryProgress { baseline, .. } = record.fact
        {
            return Ok(baseline);
        }
    }
    snapshot_baseline(transaction)
}

pub(super) fn begin_recovery(
    ledger: &SqliteLedger,
    at: UtcInstant,
    evidence: RecoveryEvidence,
) -> HistoryWrite<HistoryReceipt> {
    ledger.history_write(|transaction| {
        let namespace = storage::namespace(transaction)?;
        let baseline = snapshot_baseline(transaction)?;
        if !baseline.is_valid_for(namespace) {
            return Err(storage::conflict());
        }
        transaction
            .execute(
                "INSERT INTO history_runs (kind, started_at) VALUES (?1, ?2)",
                params![super::codec::fact::run_kind(RunKind::Recover), at.secs],
            )
            .map_err(map)?;
        let run = RunId::new(
            namespace,
            storage::positive(transaction.last_insert_rowid())?,
        );
        let started = storage::record(
            transaction,
            run,
            at,
            HistoryFact::RunStarted(RunKind::Recover),
        )?;
        let progress = storage::record(
            transaction,
            run,
            at,
            HistoryFact::RecoveryProgress { evidence, baseline },
        )?;
        Ok(HistoryReceipt {
            run,
            records: vec![started, progress],
            completed: vec![],
        })
    })
}
