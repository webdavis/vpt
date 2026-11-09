use super::read;
use crate::ledger::occasion::{codec, validate::check};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(in crate::ledger::sqlite) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.occasion else {
        return Ok(());
    };
    let row = &change.occasion;
    let key = row.key.encoded();
    let current = read::one(connection, "provider_key = ?1", &key)?;
    check(change, &current, facts)?;
    connection.execute(
        "INSERT INTO occasions (id, provider_key, source, at, duration_secs, title, participants, tags, at_rfc3339)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(provider_key) DO UPDATE SET at=excluded.at, duration_secs=excluded.duration_secs,
         title=excluded.title, participants=excluded.participants, tags=excluded.tags, at_rfc3339=excluded.at_rfc3339",
        params![row.id.as_str(), key, row.key.source().as_str(), row.details.at.unix_second().secs,
            row.details.duration_secs.map(|n| n as i64), row.details.title,
            codec::participants(&row.details.participants)?, codec::tags(&row.details.tags)?, row.details.at.as_str()],
    ).map_err(super::super::map)?;
    Ok(())
}
