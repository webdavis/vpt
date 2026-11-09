use super::super::map;
use crate::ledger::flags::resolution::{check, check_batch, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{review::Resolution, run::HistoryFact, time::UtcInstant};

pub(in crate::ledger::sqlite) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    check_batch(delta, facts)?;
    let Some(change) = &delta.resolution else {
        return Ok(());
    };
    let snapshot = super::super::verification::state::review(connection, &change.recording)?;
    if snapshot.accepted.is_none() {
        return Err(conflict());
    }
    let flag = snapshot
        .flags
        .iter()
        .find(|flag| flag.identifier == change.flag)
        .ok_or_else(conflict)?;
    check(change, flag, facts)?;
    let text = match &change.resolution {
        Resolution::Corrected { text, .. } => Some(text.as_str()),
        _ => None,
    };
    let changed = connection.execute("UPDATE flags SET state=?1, resolution_text=?2, resolved_at=?3 WHERE recording=?4 AND identifier=?5",
        params![change.resolution.kind().ok_or_else(conflict)?.as_str(), text, change.resolution.at().ok_or_else(conflict)?.secs,
            change.recording.as_str(), change.flag.hex()]).map_err(map)?;
    if changed != 1 {
        return Err(conflict());
    }
    crate::ledger::flags::budget::check(super::all(connection, &change.recording)?.iter())?;
    Ok(())
}
