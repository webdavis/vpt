use crate::ledger::relations::{check, codec, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(crate) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.relations else {
        return Ok(());
    };
    let current = super::read(connection, &change.recording)?;
    let relations = check(change, &current, facts)?;
    let rows = codec::encode(&relations)?;
    let current_ambiguities = super::ambiguities::read(connection, &change.recording)?;
    let ambiguities = crate::ledger::relations::ambiguities::prepare(
        change.ambiguities.as_ref(),
        &current_ambiguities,
    )?;
    let changed = current != relations
        || ambiguities
            .as_ref()
            .is_some_and(|next| *next != current_ambiguities);
    let affected = connection.execute("UPDATE recordings SET stage_note = CASE WHEN ?2 AND stage_note != 'expired' THEN 'pending' ELSE stage_note END WHERE id = ?1", params![change.recording.as_str(), changed]).map_err(super::super::map)?;
    if affected != 1 {
        return Err(conflict());
    }
    connection
        .execute(
            "DELETE FROM relations WHERE recording = ?1",
            [change.recording.as_str()],
        )
        .map_err(super::super::map)?;
    for row in rows {
        connection.execute("INSERT INTO relations (recording, kind, target, state, provenance, rule, ranges) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![change.recording.as_str(), row.kind, row.target, row.state, row.provenance, row.rule, row.ranges]).map_err(super::super::map)?;
    }
    if let Some(ambiguities) = ambiguities {
        super::ambiguities::write(connection, &change.recording, &ambiguities)?;
    }
    Ok(())
}
