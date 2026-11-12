use super::*;
use rusqlite::{Connection, OptionalExtension, params};

pub(in crate::ledger) fn apply(connection: &Connection, target: &Path) -> Result<(), LedgerError> {
    let target = target.to_str().ok_or_else(crate::ledger::note::invalid)?;
    connection.execute("UPDATE recordings SET stage_note='succeeded' WHERE id IN (SELECT CASE WHEN length(CAST(n.recording AS BLOB)) <= 30 THEN n.recording END FROM owned_notes n JOIN dirty_publications p ON p.target=n.target AND p.intended=n.digest WHERE n.target=?1 AND n.stage='transcript')", [target]).map_err(super::super::map)?;
    let id: Option<String> = connection.query_row("SELECT CASE WHEN length(CAST(n.recording AS BLOB)) <= 30 THEN n.recording END FROM owned_notes n JOIN dirty_publications p ON p.target=n.target AND p.intended=n.digest WHERE n.target=?1 AND n.stage='analysis'", [target], |row| row.get(0)).optional().map_err(super::super::map)?;
    let Some(id) = id else { return Ok(()) };
    let id = RecordingId::parse(&id).map_err(|_| crate::ledger::note::invalid())?;
    let review = super::super::transcript::read_with_flags(connection, &id)?;
    let proposal = super::super::proposals::read(connection, &id)?;
    let source = super::read::recording(connection, &id, NoteStage::Transcript)?;
    let current = crate::ledger::proposal::is_current(
        proposal.as_ref(),
        review.accepted.as_ref(),
        source.as_ref(),
        &review.flags,
    )?;
    connection
        .execute(
            "UPDATE recordings SET stage_analysis=?1 WHERE id=?2 AND stage_analysis NOT IN ('failed','disabled')",
            params![if current { "succeeded" } else { "pending" }, id.as_str()],
        )
        .map_err(super::super::map)?;
    Ok(())
}
