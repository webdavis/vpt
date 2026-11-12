use super::super::migrations;
use crate::ledger::tags::{check_learned, invalid};
use rusqlite::Connection;
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::identity::RecordingId;

pub(crate) fn names(connection: &Connection) -> Result<Vec<(RecordingId, String)>, LedgerError> {
    if migrations::current(connection)? < 6 {
        return Ok(vec![]);
    }
    let mut statement = connection
        .prepare("SELECT recording, tag FROM tags ORDER BY recording, ordinal")
        .map_err(|_| invalid())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| invalid())?;
    rows.map(|row| {
        let (recording, name) = row.map_err(|_| invalid())?;
        Ok((RecordingId::parse(&recording).map_err(|_| invalid())?, name))
    })
    .collect()
}

pub(crate) fn keys(connection: &Connection) -> Result<Vec<String>, LedgerError> {
    if migrations::current(connection)? < 17 {
        return Ok(vec![]);
    }
    let mut statement = connection
        .prepare("SELECT key FROM learned_tags ORDER BY key")
        .map_err(|_| invalid())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| invalid())?;
    rows.map(|row| row.map_err(|_| invalid())).collect()
}

pub(crate) fn apply(connection: &Connection, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(change) = &delta.learned_tags else {
        return Ok(());
    };
    check_learned(change)?;
    for key in &change.forgotten {
        connection
            .execute("DELETE FROM learned_tags WHERE key = ?1", [key])
            .map_err(super::super::map)?;
    }
    for key in &change.learned {
        connection
            .execute(
                "INSERT OR IGNORE INTO learned_tags (key) VALUES (?1)",
                [key],
            )
            .map_err(super::super::map)?;
    }
    Ok(())
}
