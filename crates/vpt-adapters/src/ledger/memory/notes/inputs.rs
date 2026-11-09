use super::super::{MemoryLedger, with};
use vpt_application::ports::{LedgerError, NoteInputs, NoteInputsLedger};
use vpt_domain::identity::RecordingId;

impl NoteInputsLedger for MemoryLedger {
    fn note_inputs(&self, id: &RecordingId) -> Result<Option<NoteInputs>, LedgerError> {
        with(self, |state| {
            let relations = crate::ledger::relations::resolve(
                super::super::relations::read(state, id),
                |target| {
                    Ok(state
                        .notes
                        .iter()
                        .find(|note| {
                            note.recording == *target
                                && note.stage == vpt_application::ports::NoteStage::Transcript
                        })
                        .map(|note| note.target.clone()))
                },
            )?;
            let review = super::super::verification::state::review(state, id)?;
            Ok(state
                .recordings
                .iter()
                .find(|row| row.id == *id)
                .map(|record| NoteInputs {
                    proposal: state
                        .proposals
                        .iter()
                        .find(|row| row.recording == *id)
                        .cloned(),
                    derived_mentions: None,
                    mention_ambiguities: super::super::relations::ambiguities(state, id),
                    relations,
                    tags: state
                        .tags
                        .iter()
                        .find(|(recording, _)| recording == id)
                        .map(|(_, tags)| tags.clone()),
                    record: record.clone(),
                    predecessor: state
                        .recordings
                        .iter()
                        .filter(|row| {
                            (row.captured_at, row.id.as_str())
                                < (record.captured_at, record.id.as_str())
                        })
                        .max_by_key(|row| (row.captured_at, row.id.as_str()))
                        .map(|previous| vpt_application::ports::NotePredecessor {
                            record: previous.clone(),
                            path: state
                                .notes
                                .iter()
                                .find(|note| {
                                    note.recording == previous.id
                                        && note.stage
                                            == vpt_application::ports::NoteStage::Transcript
                                })
                                .map(|note| note.target.clone()),
                        }),
                    review,
                    notes: state
                        .notes
                        .iter()
                        .filter(|row| row.recording == *id)
                        .cloned()
                        .collect(),
                }))
        })?
    }
}
