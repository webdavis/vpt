use super::{MemoryLedger, State, with};
use std::path::Path;
use vpt_application::ports::{EngineArtifact, EngineOutputLedger, LedgerError, WorkDelta};

impl EngineOutputLedger for MemoryLedger {
    fn engine_output(&self, target: &Path) -> Result<Option<EngineArtifact>, LedgerError> {
        with(self, |state| {
            state
                .engine_outputs
                .iter()
                .find(|row| row.target == target)
                .cloned()
        })
    }
}

pub(super) fn check(state: &State, delta: &WorkDelta) -> Result<(), LedgerError> {
    super::super::engine_output::check_batch(delta, |target| {
        Ok((
            state
                .engine_outputs
                .iter()
                .find(|row| row.target == target)
                .map(|row| row.digest),
            state.publications.iter().any(|row| row.target == target)
                || state
                    .intents
                    .iter()
                    .any(|(row, done)| done.is_none() && row.path == target),
        ))
    })
}

pub(super) fn apply(state: &mut State, delta: &WorkDelta) {
    for output in &delta.engine_outputs {
        match state
            .engine_outputs
            .iter_mut()
            .find(|row| row.target == output.target)
        {
            Some(row) => *row = output.clone(),
            None => state.engine_outputs.push(output.clone()),
        }
    }
}

#[cfg(test)]
mod tests;
