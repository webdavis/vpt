use super::State;
use crate::ledger::eligibility;
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    if let Some(change) = eligibility::check(delta, facts)? {
        let row = state
            .recordings
            .iter_mut()
            .find(|row| row.id == change.recording && row.stages.synthesis == change.expected)
            .ok_or_else(eligibility::conflict)?;
        row.stages.synthesis = change.next;
    }
    Ok(())
}
