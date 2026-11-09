use super::super::State;
use crate::ledger::flags::resolution::{check, check_batch, conflict};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

pub(in crate::ledger::memory) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    check_batch(delta, facts)?;
    let Some(change) = &delta.resolution else {
        return Ok(());
    };
    if !state
        .transcripts
        .iter()
        .any(|accepted| accepted.recording == change.recording)
    {
        return Err(conflict());
    }
    let current = super::super::verification::state::review(state, &change.recording)?;
    if !current
        .flags
        .iter()
        .any(|flag| flag.identifier == change.flag)
    {
        return Err(conflict());
    }
    let (_, flag) = state
        .flags
        .iter_mut()
        .find(|(id, flag)| id == &change.recording && flag.identifier == change.flag)
        .ok_or_else(conflict)?;
    check(change, flag, facts)?;
    flag.resolution = change.resolution.clone();
    crate::ledger::flags::budget::check(
        state
            .flags
            .iter()
            .filter(|(id, _)| id == &change.recording)
            .map(|(_, flag)| flag),
    )?;
    Ok(())
}
