use super::super::migrations;
use crate::ledger::tags::{MAX_BYTES, codec, invalid, validate};
use rusqlite::{Connection, OptionalExtension};
use vpt_application::ports::LedgerError;
use vpt_domain::{identity::RecordingId, tags::Tag};

pub(crate) fn recording(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Option<Vec<Tag>>, LedgerError> {
    if migrations::current(connection)? < 6 {
        return Ok(None);
    }
    let summary = connection.query_row(
        "SELECT tags_initialized, (SELECT count(*) FROM tags WHERE recording = ?1), (SELECT coalesce(sum(length(CAST(tag AS BLOB)) + length(CAST(provenance AS BLOB)) + length(CAST(state AS BLOB))), 0) FROM tags WHERE recording = ?1) FROM recordings WHERE id = ?1",
        [id.as_str()], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    ).optional().map_err(|_| invalid())?;
    let Some((initialized, count, bytes)) = summary else {
        return Ok(None);
    };
    if initialized == 0 && count == 0 {
        return Ok(None);
    }
    if initialized != 1 || !(0..=256).contains(&count) || !(0..=MAX_BYTES as i64).contains(&bytes) {
        return Err(invalid());
    }
    let mut statement = connection.prepare("SELECT CASE WHEN typeof(tag) = 'text' AND length(CAST(tag AS BLOB)) <= 262144 THEN tag END, CASE WHEN typeof(state) = 'text' AND length(CAST(state AS BLOB)) <= 9 THEN state END, provenance, updated_at, ordinal FROM tags WHERE recording = ?1 ORDER BY ordinal").map_err(|_| invalid())?;
    let rows = statement
        .query_map([id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|_| invalid())?;
    let mut tags = Vec::new();
    for (position, row) in rows.enumerate() {
        let (name, state, provenance, at, ordinal) = row.map_err(|_| invalid())?;
        if ordinal != position as i64 || tags.len() == 256 {
            return Err(invalid());
        }
        tags.push(codec::decode_row(name, &state, &provenance, at)?);
    }
    validate(&tags)?;
    Ok(Some(tags))
}
