use super::LedgerError;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;
use vpt_domain::transcript::Transcript;

#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedTranscript {
    pub checker_engine: Option<vpt_domain::transcript::EngineMetadata>,
    pub recording: RecordingId,
    pub transcript: Transcript,
    pub requested_language: String,
    pub duration: f64,
    pub accepted_at: UtcInstant,
    pub lexical_classification: bool,
    pub from_checker: bool,
    pub uncertainty: vpt_domain::uncertainty::Classification,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TranscriptionChange {
    Accept(Box<AcceptedTranscript>),
    Fail(RecordingId),
}

impl TranscriptionChange {
    pub fn recording(&self) -> &RecordingId {
        match self {
            Self::Accept(accepted) => &accepted.recording,
            Self::Fail(recording) => recording,
        }
    }
}

pub trait TranscriptLedger {
    fn accepted_transcript(
        &self,
        id: &RecordingId,
    ) -> Result<Option<AcceptedTranscript>, LedgerError>;
}

pub trait FlagLedger {
    fn transcript_flags(
        &self,
        id: &RecordingId,
    ) -> Result<Vec<vpt_domain::review::ReviewFlag>, LedgerError>;
}
