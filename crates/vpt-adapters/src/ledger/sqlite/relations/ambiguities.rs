use crate::ledger::relations::{MAX_BYTES, MAX_ITEMS, ambiguities, invalid, ranges};
use rusqlite::{Connection, params};
use vpt_application::ports::LedgerError;
use vpt_domain::{identity::RecordingId, relations::MentionAmbiguity};

pub(in crate::ledger::sqlite) fn read(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Vec<MentionAmbiguity>, LedgerError> {
    if super::super::migrations::current(connection)? < 8 {
        return Ok(vec![]);
    }
    let (count, bytes): (i64,i64) = connection.query_row("SELECT count(*), coalesce(sum(length(CAST(term AS BLOB)) + length(CAST(ranges AS BLOB))), 0) FROM mention_ambiguities WHERE recording = ?1", [id.as_str()], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|_| invalid())?;
    if !(0..=MAX_ITEMS as i64).contains(&count) || !(0..=MAX_BYTES as i64).contains(&bytes) {
        return Err(invalid());
    }
    let mut statement = connection.prepare("SELECT CASE WHEN length(CAST(term AS BLOB)) <= 262144 THEN term END, ranges FROM mention_ambiguities WHERE recording = ?1 ORDER BY term").map_err(|_| invalid())?;
    let rows = statement
        .query_map([id.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| invalid())?;
    let mut values = Vec::new();
    let mut remaining = MAX_ITEMS;
    for row in rows {
        let (term, text) = row.map_err(|_| invalid())?;
        let ranges = ranges::decode(&text)?;
        remaining = remaining.checked_sub(ranges.len()).ok_or_else(invalid)?;
        values.push(MentionAmbiguity { term, ranges });
    }
    ambiguities::ordered(values)
}

pub(super) fn write(
    connection: &Connection,
    id: &RecordingId,
    values: &[MentionAmbiguity],
) -> Result<(), LedgerError> {
    let rows = ambiguities::encode(values)?;
    connection
        .execute(
            "DELETE FROM mention_ambiguities WHERE recording = ?1",
            [id.as_str()],
        )
        .map_err(super::super::map)?;
    for (term, ranges) in rows {
        connection
            .execute(
                "INSERT INTO mention_ambiguities (recording, term, ranges) VALUES (?1, ?2, ?3)",
                params![id.as_str(), term, ranges],
            )
            .map_err(super::super::map)?;
    }
    Ok(())
}
