use super::{MemoryLedger, State, with};
use crate::ledger::relations::{check, codec, conflict};
use vpt_application::ports::{LedgerError, RelationLedger, StageState, WorkDelta};
use vpt_domain::{identity::RecordingId, relations::Relation, run::HistoryFact, time::UtcInstant};
impl RelationLedger for MemoryLedger {
    fn mention_ambiguities(
        &self,
        id: &RecordingId,
    ) -> Result<Vec<vpt_domain::relations::MentionAmbiguity>, LedgerError> {
        with(self, |state| ambiguities(state, id))
    }
    fn relations(&self, id: &RecordingId) -> Result<Vec<Relation>, LedgerError> {
        with(self, |state| read(state, id))
    }
}
pub(super) fn read(state: &State, id: &RecordingId) -> Vec<Relation> {
    state
        .relations
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, value)| value.clone())
        .unwrap_or_default()
}
pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.relations else {
        return Ok(());
    };
    let current = read(state, &change.recording);
    let relations = check(change, &current, facts)?;
    let current_ambiguities = ambiguities(state, &change.recording);
    let ambiguities = crate::ledger::relations::ambiguities::prepare(
        change.ambiguities.as_ref(),
        &current_ambiguities,
    )?;
    let changed = relations != current
        || ambiguities
            .as_ref()
            .is_some_and(|next| *next != current_ambiguities);
    codec::encode(&relations)?;
    let record = state
        .recordings
        .iter_mut()
        .find(|record| record.id == change.recording)
        .ok_or_else(conflict)?;
    if changed && record.stages.note != StageState::Expired {
        record.stages.note = StageState::Pending;
    }
    match state
        .relations
        .iter_mut()
        .find(|(id, _)| *id == change.recording)
    {
        Some((_, value)) => *value = relations,
        None => state.relations.push((change.recording.clone(), relations)),
    }
    if let Some(ambiguities) = ambiguities {
        match state
            .ambiguities
            .iter_mut()
            .find(|(id, _)| *id == change.recording)
        {
            Some((_, value)) => *value = ambiguities,
            None => state
                .ambiguities
                .push((change.recording.clone(), ambiguities)),
        }
    }
    Ok(())
}
pub(super) fn ambiguities(
    state: &State,
    id: &RecordingId,
) -> Vec<vpt_domain::relations::MentionAmbiguity> {
    state
        .ambiguities
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, value)| value.clone())
        .unwrap_or_default()
}
#[cfg(test)]
pub(crate) mod tests;
