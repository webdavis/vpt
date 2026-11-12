use super::super::map;
use super::read;
use crate::ledger::note::{self as validation, conflict, invalid};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, NoteStage, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(in crate::ledger::sqlite) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validation::check(delta, facts, |note| {
        let old = read::recording(connection, &note.recording, note.stage)?;
        let target = note.target.to_str().ok_or_else(invalid)?;
        let old_target = old
            .as_ref()
            .map(|old| old.target.as_path())
            .unwrap_or(&note.target)
            .to_str()
            .ok_or_else(invalid)?;
        let allowed: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM recordings WHERE id = ?1)
             AND NOT EXISTS(SELECT 1 FROM dirty_publications WHERE target IN (?2, ?4))
             AND NOT EXISTS(SELECT 1 FROM retention_intents WHERE path IN (?2, ?4) AND completed_at IS NULL)
             AND NOT EXISTS(SELECT 1 FROM owned_notes WHERE target = ?2 AND (recording != ?1 OR stage != ?3))",
            params![note.recording.as_str(), target, note.stage.as_str(), old_target], |row| row.get(0)
        ).map_err(map)?;
        if !allowed {
            return Err(conflict());
        }
        Ok(old)
    })?;
    for change in &delta.notes {
        let note = &change.artifact;
        if note.stage == NoteStage::Transcript {
            connection.execute("UPDATE recordings SET stage_analysis='pending' WHERE id=?1 AND stage_analysis='succeeded'
                AND EXISTS(SELECT 1 FROM owned_notes WHERE recording=?1 AND stage='transcript' AND content_digest != ?2)",
                params![note.recording.as_str(), note.content_digest.hex()]).map_err(map)?;
        }
        let tags = validation::encode_tags(&note.managed_tags)?;
        let target = note.target.to_str().ok_or_else(invalid)?;
        connection.execute("INSERT INTO owned_notes (target, recording, stage, digest, content_digest, body, managed_tags, first_written, write_offset, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(recording, stage) DO UPDATE SET target=excluded.target, digest=excluded.digest, content_digest=excluded.content_digest,
                body=excluded.body, managed_tags=excluded.managed_tags, updated_at=excluded.updated_at",
            params![target, note.recording.as_str(), note.stage.as_str(), note.digest.hex(), note.content_digest.hex(), note.text, tags, note.first_written.secs, note.write_offset.secs, note.updated_at.secs]
        ).map_err(map)?;
        let column = match note.stage {
            NoteStage::Transcript => "transcript_note_path",
            NoteStage::Analysis => "analysis_note_path",
        };
        let old_target = change
            .relocated_from
            .as_deref()
            .unwrap_or(&note.target)
            .to_str()
            .ok_or_else(invalid)?;
        let changed = connection.execute(&format!("UPDATE recordings SET {column} = ?1 WHERE id = ?2 AND ({column} IS NULL OR {column} = ?3)"), params![target, note.recording.as_str(), old_target]).map_err(map)?;
        if changed != 1 {
            return Err(conflict());
        }
    }
    Ok(())
}
