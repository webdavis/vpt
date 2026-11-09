use super::{SqliteLedger, map};
use crate::ledger::transcript::{self as codec, MAX_BODY};
use rusqlite::{Connection, OptionalExtension, params};
use vpt_application::ports::{
    AcceptedTranscript, LedgerError, TranscriptLedger, TranscriptionChange,
};
use vpt_domain::{identity::RecordingId, time::UtcInstant};

impl TranscriptLedger for SqliteLedger {
    fn accepted_transcript(
        &self,
        id: &RecordingId,
    ) -> Result<Option<AcceptedTranscript>, LedgerError> {
        self.snapshot(|connection| read(connection, id))
    }
}

pub(super) fn read(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Option<AcceptedTranscript>, LedgerError> {
    Ok(read_with_flags(connection, id)?.accepted)
}

pub(super) fn read_with_flags(
    connection: &Connection,
    id: &RecordingId,
) -> Result<vpt_application::ports::ReviewSnapshot, LedgerError> {
    let mut statement = connection.prepare(
                "SELECT CASE WHEN length(CAST(engine AS BLOB)) <= 262144 THEN engine END, CASE WHEN length(CAST(body AS BLOB)) <= ?2 THEN body END, accepted_at FROM transcripts WHERE recording = ?1"
            ).map_err(map)?;
    let row = statement
        .query_row(params![id.as_str(), MAX_BODY as i64], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .optional()
        .map_err(map)?;
    let mut accepted = row
        .map(|(engine, body, at)| {
            codec::decode(
                id.clone(),
                &engine.ok_or_else(codec::corrupt)?,
                &body.ok_or_else(codec::corrupt)?,
                UtcInstant { secs: at },
            )
        })
        .transpose()?;
    let flags = super::flags::read(connection, id)?;
    if let Some(accepted) = &mut accepted {
        accepted.uncertainty = crate::ledger::flags::classification(&flags);
        crate::ledger::flags::prepare(accepted)?;
    } else if !flags.is_empty() {
        return Err(crate::ledger::flags::corrupt());
    }
    Ok(vpt_application::ports::ReviewSnapshot {
        accepted,
        flags,
        ..Default::default()
    })
}

pub(super) fn apply(
    connection: &Connection,
    change: &TranscriptionChange,
) -> Result<(), LedgerError> {
    let stage = match change {
        TranscriptionChange::Accept(accepted) => {
            let body = codec::encode(accepted)?;
            super::flags::replace(connection, accepted)?;
            connection.execute(
                "INSERT INTO transcripts (recording, engine, body, accepted_at) VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(recording) DO UPDATE SET engine=excluded.engine, body=excluded.body, accepted_at=excluded.accepted_at",
                params![accepted.recording.as_str(), accepted.transcript.engine.name, body, accepted.accepted_at.secs],
            ).map_err(map)?;
            connection
                .execute(
                    "UPDATE recordings SET language = ?1, engines = ?2, stage_note = CASE WHEN stage_note = 'expired' THEN 'expired' ELSE 'pending' END WHERE id = ?3",
                    params![
                        accepted.requested_language,
                        accepted.transcript.engine.name,
                        accepted.recording.as_str()
                    ],
                )
                .map_err(map)?;
            "succeeded"
        }
        TranscriptionChange::Fail(_) => "failed",
    };
    let changed = connection
        .execute(
            "UPDATE recordings SET stage_transcribe = ?1 WHERE id = ?2",
            params![stage, change.recording().as_str()],
        )
        .map_err(map)?;
    if changed != 1 {
        return Err(LedgerError::Conflict(
            "unknown transcription recording".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
