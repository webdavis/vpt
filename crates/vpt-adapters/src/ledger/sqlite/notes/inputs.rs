use super::super::{SqliteLedger, recordings};
use vpt_application::ports::{LedgerError, NoteInputs, NoteInputsLedger, NoteStage};
use vpt_domain::identity::RecordingId;

impl NoteInputsLedger for SqliteLedger {
    fn note_inputs(&self, id: &RecordingId) -> Result<Option<NoteInputs>, LedgerError> {
        self.snapshot(|connection| {
            let Some(record) = recordings::one_record(connection, "id", id.as_str())? else {
                return Ok(None);
            };
            let review = super::super::verification::state::review(connection, id)?;
            let mut notes = Vec::new();
            for stage in [NoteStage::Transcript, NoteStage::Analysis] {
                if let Some(note) = super::read::recording(connection, id, stage)? {
                    notes.push(note);
                }
            }
            let relations = crate::ledger::relations::resolve(
                super::super::relations::read(connection, id)?,
                |target| super::path::read(connection, target, NoteStage::Transcript),
            )?;
            let predecessor = recordings::predecessor(connection, &record)?
                .map(|record| {
                    let path = super::path::read(connection, &record.id, NoteStage::Transcript)?;
                    Ok::<_, LedgerError>(vpt_application::ports::NotePredecessor { record, path })
                })
                .transpose()?;
            Ok(Some(NoteInputs {
                proposal: super::super::proposals::read(connection, id)?,
                derived_mentions: None,
                mention_ambiguities: super::super::relations::ambiguities::read(connection, id)?,
                relations,
                tags: super::super::tags::read(connection, id)?,
                predecessor,
                record,
                review,
                notes,
            }))
        })
    }
}
