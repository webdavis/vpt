use super::{MemoryLedger, State, with};
use crate::ledger::vocabulary::{check, conflict};
use std::path::Path;
use vpt_application::ports::{LedgerError, VocabularyLedger, VocabularyPublication, WorkDelta};
use vpt_domain::{run::HistoryFact, time::UtcInstant};

impl VocabularyLedger for MemoryLedger {
    fn vocabulary_publication(
        &self,
        target: &Path,
    ) -> Result<Option<VocabularyPublication>, LedgerError> {
        with(self, |state| {
            state
                .vocabulary
                .iter()
                .find(|row| row.entry.target == target)
                .cloned()
        })
    }
}

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    check(delta, facts)?;
    let Some(value) = &delta.vocabulary else {
        return Ok(());
    };
    let target = &value.entry.target;
    if state.publications.iter().any(|row| &row.target == target)
        || state.notes.iter().any(|row| &row.target == target)
        || state.engine_outputs.iter().any(|row| &row.target == target)
        || state.recordings.iter().any(|row| &row.audio_path == target)
        || state
            .intents
            .iter()
            .any(|(row, done)| done.is_none() && &row.path == target)
    {
        return Err(conflict());
    }
    super::upsert_publication(state, &value.entry)?;
    state.vocabulary.push(value.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    macro_rules! case {
        ($name:ident) => {
            #[test]
            fn $name() {
                crate::ledger::contract::known_terms::$name(&super::MemoryLedger::new());
            }
        };
    }
    crate::ledger::contract::known_terms::cases!(case);
}
