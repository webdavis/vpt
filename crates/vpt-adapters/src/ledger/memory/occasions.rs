use super::{MemoryLedger, State, with};
use crate::ledger::occasion::validate::{check, conflict};
use vpt_application::ports::{LedgerError, OccasionLedger, OccasionRecord, WorkDelta};
use vpt_domain::{
    occasion::{OccasionId, OccasionKey},
    run::HistoryFact,
    time::UtcInstant,
};

impl OccasionLedger for MemoryLedger {
    fn occasion(&self, id: &OccasionId) -> Result<Option<OccasionRecord>, LedgerError> {
        with(self, |state| {
            state.occasions.iter().find(|row| row.id == *id).cloned()
        })
    }
    fn occasion_by_key(&self, key: &OccasionKey) -> Result<Option<OccasionRecord>, LedgerError> {
        with(self, |state| {
            state.occasions.iter().find(|row| row.key == *key).cloned()
        })
    }
    fn occasions(&self) -> Result<Vec<OccasionRecord>, LedgerError> {
        with(self, |state| {
            let mut budget = crate::ledger::occasion::list::Budget::default();
            for row in &state.occasions {
                budget.admit(row)?;
            }
            let mut rows = state.occasions.clone();
            crate::ledger::occasion::list::sort(&mut rows);
            Ok(rows)
        })?
    }
}

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.occasion else {
        return Ok(());
    };
    let current = state
        .occasions
        .iter()
        .find(|row| row.key == change.occasion.key)
        .cloned();
    check(change, &current, facts)?;
    if state
        .occasions
        .iter()
        .any(|row| row.id == change.occasion.id && row.key != change.occasion.key)
    {
        return Err(conflict());
    }
    match state
        .occasions
        .iter_mut()
        .find(|row| row.key == change.occasion.key)
    {
        Some(row) => *row = change.occasion.clone(),
        None => state.occasions.push(change.occasion.clone()),
    }
    Ok(())
}
#[cfg(test)]
mod tests;
