use super::{SqliteLedger, map, storage};
use crate::ledger::sqlite::recordings;
use rusqlite::{OptionalExtension, params};
use vpt_application::ports::{HistoryReceipt, HistoryWrite, LedgerError, WorkDelta};
use vpt_domain::history::RunId;
use vpt_domain::run::{HistoryFact, RunKind};
use vpt_domain::time::UtcInstant;

pub(super) fn begin(
    ledger: &SqliteLedger,
    kind: RunKind,
    at: UtcInstant,
) -> HistoryWrite<HistoryReceipt> {
    ledger.history_write(|transaction| {
        let namespace = storage::namespace(transaction)?;
        transaction
            .execute(
                "INSERT INTO history_runs (kind, started_at) VALUES (?1, ?2)",
                params![super::codec::fact::run_kind(kind), at.secs],
            )
            .map_err(map)?;
        let run = RunId::new(
            namespace,
            storage::positive(transaction.last_insert_rowid())?,
        );
        let record = storage::record(transaction, run, at, HistoryFact::RunStarted(kind))?;
        Ok(HistoryReceipt {
            run,
            records: vec![record],
            completed: vec![],
        })
    })
}

pub(super) fn step(
    ledger: &SqliteLedger,
    run: RunId,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> HistoryWrite<HistoryReceipt> {
    ledger.history_write(|transaction| {
        if facts.is_empty() || run.namespace() != storage::namespace(transaction)? {
            return Err(storage::conflict());
        }
        let exists: Option<i64> = transaction
            .query_row(
                "SELECT sequence FROM history_runs WHERE sequence = ?1",
                [storage::stored(run.sequence())?],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage::column)?;
        if exists.is_none()
            || facts.iter().any(|(_, fact)| {
                matches!(fact, HistoryFact::RunStarted(_)) || !fact.referenced_ids_are_nonzero()
            })
        {
            return Err(storage::conflict());
        }
        recordings::apply_commit(transaction, &delta.commit)?;
        for (id, path) in &delta.source_updates {
            let path = path.to_str().ok_or_else(storage::corrupt)?;
            let changed = transaction
                .execute(
                    "UPDATE recordings SET source_path = ?1 WHERE id = ?2",
                    params![path, id.as_str()],
                )
                .map_err(map)?;
            if changed != 1 {
                return Err(storage::conflict());
            }
        }
        for target in &delta.publication_clears {
            let target = target.to_str().ok_or_else(storage::corrupt)?;
            transaction
                .execute("DELETE FROM dirty_publications WHERE target = ?1", [target])
                .map_err(map)?;
        }
        for (id, at) in &delta.audio_trash_marks {
            let changed = transaction
                .execute(
                    "UPDATE recordings SET audio_trashed_at = ?1 WHERE id = ?2",
                    params![at.secs, id.as_str()],
                )
                .map_err(map)?;
            if changed != 1 {
                return Err(storage::conflict());
            }
        }
        let records = facts
            .iter()
            .map(|(at, fact)| storage::record(transaction, run, *at, fact.clone()))
            .collect::<Result<Vec<_>, LedgerError>>()?;
        let completed = delta
            .commit
            .recordings
            .iter()
            .map(|recording| recording.id.clone())
            .collect();
        Ok(HistoryReceipt {
            run,
            records,
            completed,
        })
    })
}
