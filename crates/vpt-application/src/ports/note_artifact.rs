use super::LedgerError;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    time::{UtcInstant, UtcOffset},
};

pub use vpt_domain::note::NoteStage;

#[derive(Clone, PartialEq, Eq)]
pub struct NoteArtifact {
    pub recording: RecordingId,
    pub stage: NoteStage,
    pub target: PathBuf,
    pub digest: Sha256Digest,
    pub content_digest: Sha256Digest,
    pub text: String,
    pub managed_tags: Vec<String>,
    pub first_written: UtcInstant,
    pub write_offset: UtcOffset,
    pub updated_at: UtcInstant,
}

impl fmt::Debug for NoteArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoteArtifact")
            .field("recording", &self.recording)
            .field("stage", &self.stage)
            .field("target", &self.target)
            .field("bytes", &self.text.len())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteChange {
    pub relocated_from: Option<PathBuf>,
    pub artifact: NoteArtifact,
    pub expected_saved: Option<Sha256Digest>,
}

pub trait NoteLedger {
    fn note_path(
        &self,
        recording: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<PathBuf>, LedgerError>;

    fn note(
        &self,
        recording: &RecordingId,
        stage: NoteStage,
    ) -> Result<Option<NoteArtifact>, LedgerError>;
    fn note_at(&self, target: &Path) -> Result<Option<NoteArtifact>, LedgerError>;
    /// Visits saved notes newest captured recording first, a transcript before its analysis.
    fn visit_notes(
        &self,
        visitor: &mut dyn FnMut(NoteArtifact) -> std::ops::ControlFlow<()>,
    ) -> Result<(), LedgerError>;
}
