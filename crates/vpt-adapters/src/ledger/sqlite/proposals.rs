use super::{SqliteLedger, map};
use crate::ledger::proposal::{self as validation, codec, conflict, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use vpt_application::ports::*;
use vpt_domain::{identity::RecordingId, run::HistoryFact, time::UtcInstant};

impl ProposalLedger for SqliteLedger {
    fn accepted_proposal(&self, id: &RecordingId) -> Result<Option<AcceptedProposal>, LedgerError> {
        self.snapshot(|connection| read(connection, id))
    }
    fn grounding_basis(&self, id: &RecordingId) -> Result<Option<GroundingBasis>, LedgerError> {
        self.snapshot(|connection| {
            let review = super::transcript::read_with_flags(connection, id)?;
            review
                .accepted
                .as_ref()
                .map(|accepted| validation::basis::compute(accepted, &review.flags))
                .transpose()
        })
    }
}

pub(super) fn read(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Option<AcceptedProposal>, LedgerError> {
    let row = connection.query_row(
        "SELECT CASE WHEN length(CAST(input_digest AS BLOB))=64 THEN input_digest END,
         CASE WHEN length(CAST(body AS BLOB)) <= ?2 THEN body END, accepted_at FROM proposals WHERE recording=?1",
        params![id.as_str(), codec::MAX_BODY as i64], |row| Ok((row.get::<_,Option<String>>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,i64>(2)?))
    ).optional().map_err(map)?;
    row.map(|(digest, body, at)| {
        codec::decode(
            id.clone(),
            &digest.ok_or_else(invalid)?,
            &body.ok_or_else(invalid)?,
            UtcInstant { secs: at },
        )
    })
    .transpose()
}

pub(super) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validation::history(delta, facts)?;
    let Some(change) = &delta.proposal else {
        return Ok(());
    };
    let record = super::recordings::one_record(connection, "id", change.recording().as_str())?
        .ok_or_else(conflict)?;
    let stage = match change {
        ProposalChange::Fail { .. } => "failed",
        ProposalChange::Accept(accepted) => {
            let review = super::transcript::read_with_flags(connection, &accepted.recording)?;
            let transcript = review.accepted.as_ref().ok_or_else(conflict)?;
            let source =
                super::notes::recording(connection, &accepted.recording, NoteStage::Transcript)?
                    .ok_or_else(conflict)?;
            let blocked: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM dirty_publications WHERE target=?1)
                OR EXISTS(SELECT 1 FROM retention_intents WHERE path=?1 AND completed_at IS NULL)",
                    [source.target.to_str().ok_or_else(invalid)?],
                    |row| row.get(0),
                )
                .map_err(map)?;
            if blocked {
                return Err(conflict());
            }
            let flags = super::flags::all(connection, &accepted.recording)?;
            let prepared = validation::check(
                accepted,
                delta,
                transcript,
                &flags,
                &source,
                record.stages.note,
            )?;
            connection.execute("INSERT INTO proposals (recording,input_digest,body,accepted_at) VALUES (?1,?2,?3,?4)
                ON CONFLICT(recording) DO UPDATE SET input_digest=excluded.input_digest,body=excluded.body,accepted_at=excluded.accepted_at",
                params![accepted.recording.as_str(),accepted.input_digest.hex(),prepared.body,accepted.accepted_at.secs]).map_err(map)?;
            super::verification::replace(
                connection,
                &accepted.recording,
                NoteStage::Analysis,
                &prepared.findings,
            )?;
            super::verification::state::save(connection, &prepared.verification)?;
            "pending"
        }
    };
    connection
        .execute(
            "UPDATE recordings SET stage_analysis=?1 WHERE id=?2",
            params![stage, change.recording().as_str()],
        )
        .map_err(map)?;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
