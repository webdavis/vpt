use super::SqliteLedger;
use std::path::Path;
use vpt_application::ports::{LedgerError, NoteArtifact, NoteLedger, NoteStage};
use vpt_domain::identity::RecordingId;
mod inputs;
mod path;
pub(super) mod read;
mod write;
pub(super) use read::recording;
pub(super) use write::apply;

mod published;
pub(super) use published::apply as published;

impl NoteLedger for SqliteLedger {
    fn note_path(
        &self,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<std::path::PathBuf>, LedgerError> {
        self.read(|connection| path::read(connection, id, stage))
    }

    fn note(
        &self,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<NoteArtifact>, LedgerError> {
        self.read(|connection| read::recording(connection, id, stage))
    }
    fn note_at(&self, target: &Path) -> Result<Option<NoteArtifact>, LedgerError> {
        self.read(|connection| read::target(connection, target))
    }
    fn visit_notes(
        &self,
        visitor: &mut dyn FnMut(NoteArtifact) -> std::ops::ControlFlow<()>,
    ) -> Result<(), LedgerError> {
        self.read(|connection| read::visit(connection, visitor))
    }
}

#[cfg(test)]
pub(super) mod tests;
