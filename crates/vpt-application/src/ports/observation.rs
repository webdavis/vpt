use super::{LedgerError, NoteStage, RecordingRecord, RetentionArtifact};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    transcript::{EngineMetadata, LanguageSource},
};

pub struct TranscriptionMetadata {
    pub engines: Vec<EngineMetadata>,
    pub language: String,
    pub requested_language: String,
    pub language_source: LanguageSource,
    pub local: bool,
    pub from_checker: bool,
}

pub struct ObservedFlag {
    pub identifier: Sha256Digest,
    pub class: &'static str,
    pub artifact: Option<NoteStage>,
}

pub struct RecordingObservation {
    pub record: RecordingRecord,
    pub transcription: Option<TranscriptionMetadata>,
    pub flags: Vec<ObservedFlag>,
    pub diagnostics: Vec<ObservedFlag>,
    pub stale_artifacts: Vec<NoteStage>,
    pub artifacts: Vec<RetentionArtifact>,
}

pub trait ObservationLedger {
    /// Reads metadata from one consistent snapshot, in capture order. No content bodies escape.
    /// At most 65,536 recordings and 16 MiB of retained metadata.
    fn observe_recordings(
        &self,
        id: Option<&RecordingId>,
    ) -> Result<Vec<RecordingObservation>, LedgerError>;
}
