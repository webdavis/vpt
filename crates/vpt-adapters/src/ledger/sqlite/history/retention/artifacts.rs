use super::{codec, journal, storage};
use crate::ledger::retention_artifacts::completion::{binding, conflict, fact};
use crate::ledger::sqlite::{map, retention_artifacts};
use rusqlite::{OptionalExtension, Transaction, params};
use vpt_application::ports::*;
use vpt_domain::{
    history::{EventId, RunId},
    run::{EffectOutcome, HistoryFact, WorkOutcome},
    time::UtcInstant,
};

pub(super) fn complete(
    transaction: &Transaction<'_>,
    run: RunId,
    intent: &RetentionIntent,
    completed: Option<i64>,
    effect: EventId,
    at: UtcInstant,
) -> Result<HistoryReceipt, LedgerError> {
    let saved: Option<String> = transaction
        .query_row(
            "SELECT fact FROM history_records WHERE sequence=?1",
            [storage::stored(effect.sequence())?],
            |row| row.get(0),
        )
        .optional()
        .map_err(map)?;
    if saved.as_deref().map(codec::decode).transpose()? != Some(fact(intent)?) {
        return Err(conflict());
    }
    let artifact = binding(retention_artifacts::read(transaction)?, intent)?;
    let mut receipt = HistoryReceipt {
        run,
        records: vec![],
        completed: intent.owner.recording().cloned().into_iter().collect(),
    };
    if completed.is_some() {
        if artifact.trashed_at.is_none() {
            return Err(conflict());
        }
        return Ok(receipt);
    }
    transaction.execute("INSERT INTO artifact_expirations(path,kind,owner,digest,trashed_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(path) DO NOTHING", params![artifact.path.to_str().ok_or_else(conflict)?, artifact.kind.as_str(), artifact.owner.as_str(), artifact.digest.hex(), at.secs]).map_err(map)?;
    let stage = match intent.kind {
        ArtifactKind::Transcript => Some("stage_note"),
        ArtifactKind::Analysis => Some("stage_analysis"),
        _ => None,
    };
    if let Some(stage) = stage {
        transaction
            .execute(
                &format!("UPDATE recordings SET {stage}='expired' WHERE id=?1"),
                [intent.owner.as_str()],
            )
            .map_err(map)?;
    }
    journal::complete(transaction, intent.id, at)?;
    for event in [
        HistoryFact::EffectObserved {
            intent: effect,
            outcome: EffectOutcome::Completed,
        },
        HistoryFact::WorkStep {
            recording: intent.owner.recording().cloned(),
            outcome: WorkOutcome::Completed,
        },
    ] {
        receipt
            .records
            .push(storage::record(transaction, run, at, event)?);
    }
    Ok(receipt)
}
