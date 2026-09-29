use super::{SqliteLedger, codec, map, storage};
use crate::ledger::sqlite::retention as journal;
use rusqlite::{OptionalExtension, Transaction, params};
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, LedgerError, NewIntent, PreparedRetention, RetentionHistory,
};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, HistoryFact, WorkOutcome};
use vpt_domain::time::UtcInstant;

impl RetentionHistory for SqliteLedger {
    fn prepare_retention(&self, run: RunId, intent: &NewIntent) -> HistoryWrite<PreparedRetention> {
        self.history_write(|transaction| prepare(transaction, run, intent))
    }

    fn complete_retention(
        &self,
        run: RunId,
        intent_id: i64,
        effect_id: EventId,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt> {
        self.history_write(|transaction| complete(transaction, run, intent_id, effect_id, at))
    }
}

fn prepare(
    transaction: &Transaction<'_>,
    run: RunId,
    intent: &NewIntent,
) -> Result<PreparedRetention, LedgerError> {
    let intent_id = journal::insert(transaction, intent)?;
    let saved = storage::record(
        transaction,
        run,
        intent.recorded_at,
        HistoryFact::EffectIntent {
            kind: EffectKind::RetentionTrash,
            reference: EffectReference::RetentionIntent {
                id: storage::positive(intent_id)?,
                path: intent.path.clone(),
            },
        },
    )?;
    Ok(PreparedRetention {
        intent_id,
        effect_id: saved.id,
        receipt: HistoryReceipt {
            run,
            records: vec![saved],
            completed: vec![],
        },
    })
}

fn complete(
    transaction: &Transaction<'_>,
    run: RunId,
    intent_id: i64,
    effect_id: EventId,
    at: UtcInstant,
) -> Result<HistoryReceipt, LedgerError> {
    if run.namespace() != storage::namespace(transaction)?
        || effect_id.namespace() != run.namespace()
    {
        return Err(storage::conflict());
    }
    let run_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM history_runs WHERE sequence = ?1)",
            [storage::stored(run.sequence())?],
            |row| row.get(0),
        )
        .map_err(storage::column)?;
    if !run_exists {
        return Err(storage::conflict());
    }
    let found = transaction.query_row(
        "SELECT id, artifact_kind, recording, path, expected, recorded_at, completed_at FROM retention_intents WHERE id = ?1",
        [intent_id], |row| Ok((journal::intent_from_row(row), row.get::<_, Option<i64>>(6)?)),
    ).optional().map_err(storage::column)?.ok_or_else(storage::conflict)?;
    let (intent, completed) = (found.0?, found.1);
    let fact: Option<String> = transaction
        .query_row(
            "SELECT fact FROM history_records WHERE sequence = ?1",
            [storage::stored(effect_id.sequence())?],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage::column)?;
    let expected = HistoryFact::EffectIntent {
        kind: EffectKind::RetentionTrash,
        reference: EffectReference::RetentionIntent {
            id: storage::positive(intent.id)?,
            path: intent.path.clone(),
        },
    };
    if fact.as_deref().map(codec::decode).transpose()? != Some(expected) {
        return Err(storage::conflict());
    }
    let pinned: Option<(String, String, Option<i64>)> = transaction
        .query_row(
            "SELECT audio_path, digest, audio_trashed_at FROM recordings WHERE id = ?1",
            [intent.recording.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(storage::column)?;
    let (path, digest, marked) = pinned.ok_or_else(storage::conflict)?;
    if std::path::Path::new(&path) != intent.path || digest != intent.expected.hex() {
        return Err(storage::conflict());
    }
    let mut receipt = HistoryReceipt {
        run,
        records: vec![],
        completed: vec![intent.recording.clone()],
    };
    if completed.is_some() {
        if marked.is_none() {
            return Err(storage::conflict());
        }
        return Ok(receipt);
    }
    transaction
        .execute(
            "UPDATE recordings SET audio_trashed_at = COALESCE(audio_trashed_at, ?1) WHERE id = ?2",
            params![at.secs, intent.recording.as_str()],
        )
        .map_err(map)?;
    journal::complete(transaction, intent_id, at)?;
    for fact in [
        HistoryFact::EffectObserved {
            intent: effect_id,
            outcome: EffectOutcome::Completed,
        },
        HistoryFact::WorkStep {
            recording: Some(intent.recording),
            outcome: WorkOutcome::Completed,
        },
    ] {
        receipt
            .records
            .push(storage::record(transaction, run, at, fact)?);
    }
    Ok(receipt)
}

#[cfg(test)]
pub(in crate::ledger::sqlite) mod tests;
