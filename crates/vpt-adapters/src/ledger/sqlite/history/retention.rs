mod artifacts;
mod complete;
use super::{SqliteLedger, codec, storage};
use crate::ledger::sqlite::retention as journal;
use rusqlite::Transaction;
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, LedgerError, NewIntent, PreparedRetention, RetentionHistory,
};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{EffectKind, EffectReference, HistoryFact};
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
        self.history_write(|transaction| {
            complete::apply(transaction, run, intent_id, effect_id, at)
        })
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

#[cfg(test)]
pub(in crate::ledger::sqlite) mod tests;
