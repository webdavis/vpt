use super::RuntimeLedger;
use std::path::{Path, PathBuf};
use vpt_application::ports::{LedgerError, NoteArtifact, NoteLedger, NoteStage};
use vpt_domain::identity::RecordingId;

impl RuntimeLedger {
    fn notes(&self) -> &dyn NoteLedger {
        match self {
            Self::Sqlite(ledger) => ledger,
            Self::Empty(ledger) => ledger,
        }
    }
}
impl NoteLedger for RuntimeLedger {
    fn note_path(
        &self,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<PathBuf>, LedgerError> {
        self.notes().note_path(id, stage)
    }
    fn note(
        &self,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<NoteArtifact>, LedgerError> {
        self.notes().note(id, stage)
    }
    fn note_at(&self, target: &Path) -> Result<Option<NoteArtifact>, LedgerError> {
        self.notes().note_at(target)
    }
}

impl vpt_application::ports::NoteInputsLedger for RuntimeLedger {
    fn note_inputs(
        &self,
        id: &RecordingId,
    ) -> Result<Option<vpt_application::ports::NoteInputs>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.note_inputs(id),
            Self::Empty(ledger) => ledger.note_inputs(id),
        }
    }
}
