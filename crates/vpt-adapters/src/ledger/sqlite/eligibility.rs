use crate::ledger::eligibility;
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(super) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    if let Some(change) = eligibility::check(delta, facts)? {
        let changed = connection
            .execute(
                "UPDATE recordings SET stage_synthesis=?1 WHERE id=?2 AND stage_synthesis=?3",
                params![
                    change.next.as_str(),
                    change.recording.as_str(),
                    change.expected.as_str()
                ],
            )
            .map_err(super::map)?;
        if changed != 1 {
            return Err(eligibility::conflict());
        }
    }
    Ok(())
}
