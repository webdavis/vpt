use crate::ledger::tags::{check, check_rename, codec, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(crate) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.tags else {
        return Ok(());
    };
    let current = super::read(connection, &change.recording)?;
    check(change, &current, facts)?;
    codec::encode(&change.tags)?;
    let changed = current.as_ref() != Some(&change.tags);
    let rows = connection.execute("UPDATE recordings SET tags_initialized = 1, stage_note = CASE WHEN ?2 AND stage_note != 'expired' THEN 'pending' ELSE stage_note END WHERE id = ?1", params![change.recording.as_str(), changed]).map_err(super::super::map)?;
    if rows != 1 {
        return Err(conflict());
    }
    connection
        .execute(
            "DELETE FROM tags WHERE recording = ?1",
            [change.recording.as_str()],
        )
        .map_err(super::super::map)?;
    for (ordinal, tag) in change.tags.iter().enumerate() {
        connection.execute("INSERT INTO tags (recording, tag, state, provenance, updated_at, ordinal) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![change.recording.as_str(), tag.name, codec::state(tag.state), codec::provenance(&tag.provenance)?, tag.updated_at.map(|at| at.secs), ordinal as i64]).map_err(super::super::map)?;
    }
    Ok(())
}

pub(crate) fn apply_rename(connection: &Connection, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(rename) = &delta.tag_rename else {
        return Ok(());
    };
    check_rename(rename)?;
    connection
        .execute(
            "INSERT INTO tag_renames (old, new, renamed_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(old) DO UPDATE SET new = excluded.new, renamed_at = excluded.renamed_at",
            params![rename.old, rename.new, rename.at.secs],
        )
        .map_err(super::super::map)?;
    Ok(())
}
