use super::{MemoryLedger, with};
use vpt_application::ports::*;
impl BriefInputsLedger for MemoryLedger {
    fn visit_brief_inputs(
        &self,
        visitor: &mut dyn FnMut(BriefInput) -> Result<(), LedgerError>,
    ) -> Result<(), LedgerError> {
        with(self, |state| {
            if state.recordings.len() > 65_536 {
                return Err(LedgerError::Corrupt("brief inputs exceed capacity".into()));
            }
            let mut records: Vec<_> = state.recordings.iter().collect();
            records.sort_by(|a, b| {
                (a.captured_at, a.id.as_str()).cmp(&(b.captured_at, b.id.as_str()))
            });
            for recording in records {
                let id = &recording.id;
                let note = state
                    .notes
                    .iter()
                    .find(|note| {
                        recording.stages.note == StageState::Succeeded
                            && note.recording == *id
                            && note.stage == NoteStage::Transcript
                    })
                    .cloned();
                let review = if note.is_some() {
                    super::verification::state::review(state, id)?
                } else {
                    ReviewSnapshot::default()
                };
                let tags = state
                    .tags
                    .iter()
                    .find(|(key, _)| key == id)
                    .map(|(_, rows)| rows.clone())
                    .unwrap_or_default();
                let relations = super::relations::read(state, id);
                visitor(BriefInput {
                    recording: recording.clone(),
                    note,
                    review,
                    tags,
                    relations,
                })?;
            }
            Ok(())
        })?
    }
}
