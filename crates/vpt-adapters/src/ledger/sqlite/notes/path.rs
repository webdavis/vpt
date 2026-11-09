use crate::ledger::note::{invalid, validate_target};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;
use vpt_application::ports::{LedgerError, NoteStage};
use vpt_domain::identity::RecordingId;

pub(super) fn read(
    connection: &Connection,
    id: &RecordingId,
    stage: NoteStage,
) -> Result<Option<PathBuf>, LedgerError> {
    if super::super::migrations::current(connection)? < 4 {
        return Ok(None);
    }
    let target: Option<String> = connection.query_row(
        "SELECT CASE WHEN length(CAST(target AS BLOB)) <= 65536 THEN target END FROM owned_notes WHERE recording = ?1 AND stage = ?2",
        params![id.as_str(), stage.as_str()], |row| {
            if row.get_ref(0)?.as_bytes()?.len() > 65_536 { return Err(rusqlite::Error::InvalidQuery); }
            row.get(0)
        }
    ).optional().map_err(|error| match error {
        rusqlite::Error::InvalidQuery | rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::FromSqlConversionFailure(..) => invalid(),
        other => super::super::map(other),
    })?;
    target
        .map(|value| {
            let path = PathBuf::from(value);
            validate_target(&path)?;
            Ok(path)
        })
        .transpose()
}
