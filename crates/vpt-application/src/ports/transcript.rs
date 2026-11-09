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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReviewSnapshot {
    pub verified_claims: Vec<super::VerifiedClaims>,
    /// Saved decisions excluded from current review until the artifact is checked again.
    pub stale_findings: Vec<vpt_domain::review::ReviewFlag>,
    pub stale_artifacts: Vec<super::NoteStage>,
    pub accepted: Option<AcceptedTranscript>,
    pub flags: Vec<vpt_domain::review::ReviewFlag>,
}

impl ReviewSnapshot {
    pub fn flags_for_recheck(&self) -> Vec<vpt_domain::review::ReviewFlag> {
        self.flags
            .iter()
            .chain(&self.stale_findings)
            .cloned()
            .collect()
    }

    pub fn invalidate_verification(&mut self) {
        for stage in [super::NoteStage::Transcript, super::NoteStage::Analysis] {
            self.invalidate_artifact(stage);
        }
    }

    pub fn invalidate_artifact(&mut self, stage: super::NoteStage) {
        use vpt_domain::review::FlagContent;
        self.verified_claims
            .retain(|claims| claims.artifact != stage);
        if !self.stale_artifacts.contains(&stage) {
            self.stale_artifacts.push(stage);
        }
        let mut current = Vec::with_capacity(self.flags.len());
        for flag in std::mem::take(&mut self.flags) {
            if matches!(&flag.content, FlagContent::Verification(finding) if finding.artifact == stage)
            {
                self.stale_findings.push(flag);
            } else {
                current.push(flag);
            }
        }
        self.flags = current;
    }
}

pub trait ReviewLedger {
    /// Reads recording existence, accepted words, and flags from one consistent snapshot.
    fn review_snapshot(&self, id: &RecordingId) -> Result<Option<ReviewSnapshot>, LedgerError>;
}
