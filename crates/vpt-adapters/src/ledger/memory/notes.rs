use super::{MemoryLedger, State, with};
use crate::ledger::note::{self as validation, conflict};
use std::path::Path;
use vpt_application::ports::{LedgerError, NoteArtifact, NoteLedger, NoteStage, WorkDelta};
use vpt_domain::identity::RecordingId;
use vpt_domain::{run::HistoryFact, time::UtcInstant};

impl NoteLedger for MemoryLedger {
    fn note_path(
        &self,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<std::path::PathBuf>, LedgerError> {
        with(self, |state| {
            state
                .notes
                .iter()
                .find(|note| note.recording == *id && note.stage == stage)
                .map(|note| note.target.clone())
        })
    }

    fn note(
        &self,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<NoteArtifact>, LedgerError> {
        with(self, |state| {
            state
                .notes
                .iter()
                .find(|note| note.recording == *id && note.stage == stage)
                .cloned()
        })
    }
    fn note_at(&self, target: &Path) -> Result<Option<NoteArtifact>, LedgerError> {
        with(self, |state| {
            state
                .notes
                .iter()
                .find(|note| note.target == target)
                .cloned()
        })
    }
    fn visit_notes(
        &self,
        visitor: &mut dyn FnMut(NoteArtifact) -> std::ops::ControlFlow<()>,
    ) -> Result<(), LedgerError> {
        let mut notes: Vec<_> = with(self, |state| {
            state
                .notes
                .iter()
                .filter_map(|note| {
                    let record = state.recordings.iter().find(|r| r.id == note.recording)?;
                    Some((record.captured_at, note.clone()))
                })
                .collect()
        })?;
        notes.sort_by_key(|(at, note)| {
            (
                std::cmp::Reverse(*at),
                std::cmp::Reverse(note.recording.as_str().to_owned()),
                note.stage,
            )
        });
        for (_, note) in notes {
            if visitor(note).is_break() {
                break;
            }
        }
        Ok(())
    }
}

pub(super) fn apply(
    state: &mut State,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validation::check(delta, facts, |note| {
        let old = state
            .notes
            .iter()
            .find(|row| row.recording == note.recording && row.stage == note.stage);
        let reserved =
            |path: &Path| path == note.target || old.is_some_and(|old| old.target == path);
        if !state.recordings.iter().any(|row| row.id == note.recording)
            || state.publications.iter().any(|row| reserved(&row.target))
            || state
                .intents
                .iter()
                .any(|(intent, done)| done.is_none() && reserved(&intent.path))
            || state.notes.iter().any(|row| {
                row.target == note.target
                    && (row.recording != note.recording || row.stage != note.stage)
            })
        {
            return Err(conflict());
        }
        Ok(old.cloned())
    })?;
    for change in &delta.notes {
        validation::encode_tags(&change.artifact.managed_tags)?;
        if change.artifact.stage == NoteStage::Transcript
            && state.notes.iter().any(|old| {
                old.recording == change.artifact.recording
                    && old.stage == NoteStage::Transcript
                    && old.content_digest != change.artifact.content_digest
            })
            && let Some(record) = state
                .recordings
                .iter_mut()
                .find(|record| record.id == change.artifact.recording)
            && record.stages.analysis == vpt_application::ports::StageState::Succeeded
        {
            record.stages.analysis = vpt_application::ports::StageState::Pending;
        }
        match state.notes.iter_mut().find(|note| {
            note.recording == change.artifact.recording && note.stage == change.artifact.stage
        }) {
            Some(note) => *note = change.artifact.clone(),
            None => state.notes.push(change.artifact.clone()),
        }
    }
    Ok(())
}

mod inputs;
#[cfg(test)]
mod tests;

mod published;
pub(super) use published::apply as published;
