use crate::ledger::relations::{MAX_BYTES, MAX_ITEMS, codec, invalid, ordered};
use rusqlite::Connection;
use vpt_application::ports::LedgerError;
use vpt_domain::{identity::RecordingId, relations::Relation};

pub(crate) fn recording(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Vec<Relation>, LedgerError> {
    let (count, bytes): (i64, i64) = connection.query_row(
        "SELECT count(*), coalesce(sum(length(CAST(kind AS BLOB)) + length(CAST(target AS BLOB)) + length(CAST(state AS BLOB)) + length(CAST(provenance AS BLOB)) + coalesce(length(CAST(rule AS BLOB)), 0) + coalesce(length(CAST(ranges AS BLOB)), 0)), 0) FROM relations WHERE recording = ?1",
        [id.as_str()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| invalid())?;
    if !(0..=MAX_ITEMS as i64).contains(&count) || !(0..=MAX_BYTES as i64).contains(&bytes) {
        return Err(invalid());
    }
    let mut statement = connection.prepare("SELECT kind, target, state, provenance, rule, ranges FROM relations WHERE recording = ?1").map_err(|_| invalid())?;
    let rows = statement
        .query_map([id.as_str()], |row| {
            Ok(codec::Row {
                kind: row.get(0)?,
                target: row.get(1)?,
                state: row.get(2)?,
                provenance: row.get(3)?,
                rule: row.get(4)?,
                ranges: row.get(5)?,
            })
        })
        .map_err(|_| invalid())?;
    let mut relations = Vec::new();
    let mut remaining_ranges = MAX_ITEMS;
    for row in rows {
        if relations.len() == MAX_ITEMS {
            return Err(invalid());
        }
        let relation = codec::decode(row.map_err(|_| invalid())?)?;
        if relation.target.as_str() == id.as_str() {
            return Err(invalid());
        }
        remaining_ranges = remaining_ranges
            .checked_sub(relation.ranges.len())
            .ok_or_else(invalid)?;
        relations.push(relation);
    }
    ordered(relations)
}
