use super::super::{journal, map};
use crate::ledger::vocabulary::{check, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, VocabularyKind, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(in crate::ledger::sqlite) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    check(delta, facts)?;
    let Some(value) = &delta.vocabulary else {
        return Ok(());
    };
    let target = value.entry.target.to_str().ok_or_else(conflict)?;
    let reserved: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM dirty_publications WHERE target=?1)
        OR EXISTS(SELECT 1 FROM owned_notes WHERE target=?1)
        OR EXISTS(SELECT 1 FROM engine_outputs WHERE target=?1)
        OR EXISTS(SELECT 1 FROM recordings WHERE audio_path=?1)
        OR EXISTS(SELECT 1 FROM retention_intents WHERE path=?1 AND completed_at IS NULL)",
            [target],
            |row| row.get(0),
        )
        .map_err(map)?;
    if reserved {
        return Err(conflict());
    }
    journal::upsert_publication(connection, &value.entry)?;
    let kind = match value.kind {
        VocabularyKind::Terms => "terms",
        VocabularyKind::Tags => "tags",
    };
    connection
        .execute(
            "INSERT INTO known_term_publications(target, body, kind) VALUES (?1, ?2, ?3)",
            params![target, value.bytes, kind],
        )
        .map_err(map)?;
    Ok(())
}
