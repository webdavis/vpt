use super::*;
use crate::ledger::release::{self as release, conflict, validate};
use rusqlite::params;
use vpt_domain::run::{EffectOutcome, WorkOutcome};
pub(super) fn apply(
    c: &Transaction<'_>,
    run: RunId,
    sequence: i64,
    expected: ReleaseVersion,
    at: UtcInstant,
) -> Result<HistoryReceipt, LedgerError> {
    validate::instant(at)?;
    let exists: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM history_runs WHERE sequence=?1)",
            [storage::stored(run.sequence())?],
            |r| r.get(0),
        )
        .map_err(map)?;
    if !exists || run.namespace() != storage::namespace(c)? {
        return Err(conflict());
    }
    let row = super::super::super::releases::read::record(c, sequence)?.ok_or_else(conflict)?;
    if row.version != expected {
        return Err(conflict());
    }
    let mut receipt = HistoryReceipt {
        run,
        records: vec![],
        completed: vec![row.recording.clone()],
    };
    if row.finalized_at.is_some() {
        return Ok(receipt);
    }
    let payload =
        super::super::super::releases::read::payload(c, sequence)?.ok_or_else(conflict)?;
    for (intent, expected) in payload.intents.iter().zip(release::intents(&row)) {
        let saved: String = c.query_row("SELECT CASE WHEN length(CAST(fact AS BLOB))<=1048576 THEN fact END FROM history_records WHERE sequence=?1", [storage::stored(intent.sequence())?], |r| r.get(0)).map_err(storage::column)?;
        if codec::decode(&saved)? != expected {
            return Err(conflict());
        }
    }
    for fact in payload
        .intents
        .map(|intent| HistoryFact::EffectObserved {
            intent,
            outcome: EffectOutcome::Completed,
        })
        .into_iter()
        .chain([HistoryFact::WorkStep {
            recording: Some(row.recording),
            outcome: WorkOutcome::Completed,
        }])
    {
        receipt.records.push(storage::record(c, run, at, fact)?);
    }
    c.execute(
        "INSERT INTO release_completions (sequence,completed_at) VALUES (?1,?2)",
        params![sequence, at.secs],
    )
    .map_err(map)?;
    c.execute("DELETE FROM release_payloads WHERE sequence=?1", [sequence])
        .map_err(map)?;
    Ok(receipt)
}
