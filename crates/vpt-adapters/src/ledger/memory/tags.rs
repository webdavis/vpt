use super::{MemoryLedger, State, with};
use crate::ledger::tags::{check, check_learned, check_rename, conflict};
use vpt_application::ports::{LedgerError, TagLedger, TagRename, WorkDelta};
use vpt_domain::{
    identity::RecordingId,
    run::HistoryFact,
    tags::{self, Tag},
    time::UtcInstant,
};

impl TagLedger for MemoryLedger {
    fn tags(&self, id: &RecordingId) -> Result<Option<Vec<Tag>>, LedgerError> {
        with(self, |state| read(state, id))
    }

    fn tag_renames(&self) -> Result<Vec<TagRename>, LedgerError> {
        with(self, |state| {
            let mut renames = state.tag_renames.clone();
            renames.sort_by(|a, b| (a.at, &a.old).cmp(&(b.at, &b.old)));
            renames
        })
    }

    fn tag_states(&self) -> Result<Vec<(String, tags::State)>, LedgerError> {
        with(self, |state| {
            state
                .tags
                .iter()
                .flat_map(|(_, tags)| tags.iter().map(|tag| (tag.name.clone(), tag.state)))
                .collect()
        })
    }
    fn tag_names(&self) -> Result<Vec<(RecordingId, String)>, LedgerError> {
        with(self, |state| {
            state
                .tags
                .iter()
                .flat_map(|(recording, tags)| {
                    tags.iter().map(|tag| (recording.clone(), tag.name.clone()))
                })
                .collect()
        })
    }

    fn learned_tags(&self) -> Result<Vec<String>, LedgerError> {
        with(self, |state| state.learned_tags.iter().cloned().collect())
    }
}

pub(super) fn apply_learned(state: &mut State, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(change) = &delta.learned_tags else {
        return Ok(());
    };
    check_learned(change)?;
    for key in &change.forgotten {
        state.learned_tags.remove(key);
    }
    state.learned_tags.extend(change.learned.iter().cloned());
    Ok(())
}

pub(super) fn apply_rename(state: &mut State, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(rename) = &delta.tag_rename else {
        return Ok(());
    };
    check_rename(rename)?;
    state.tag_renames.retain(|saved| saved.old != rename.old);
    state.tag_renames.push(rename.clone());
    Ok(())
}

fn read(state: &State, id: &RecordingId) -> Option<Vec<Tag>> {
    state
        .tags
        .iter()
        .find(|(recording, _)| recording == id)
        .map(|(_, tags)| tags.clone())
}

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.tags else {
        return Ok(());
    };
    if !state
        .recordings
        .iter()
        .any(|record| record.id == change.recording)
    {
        return Err(conflict());
    }
    let current = read(state, &change.recording);
    check(change, &current, facts)?;
    crate::ledger::tags::codec::encode(&change.tags)?;
    if current.as_ref() != Some(&change.tags) {
        let record = state
            .recordings
            .iter_mut()
            .find(|record| record.id == change.recording)
            .ok_or_else(conflict)?;
        if record.stages.note != vpt_application::ports::StageState::Expired {
            record.stages.note = vpt_application::ports::StageState::Pending;
        }
    }
    match state
        .tags
        .iter_mut()
        .find(|(recording, _)| *recording == change.recording)
    {
        Some((_, tags)) => *tags = change.tags.clone(),
        None => state
            .tags
            .push((change.recording.clone(), change.tags.clone())),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
