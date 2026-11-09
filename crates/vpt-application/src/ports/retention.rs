use super::ArtifactOwner;
use super::ledger::LedgerError;
use std::path::PathBuf;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::UtcInstant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Audio,
    EngineOutput,
    Transcript,
    Analysis,
    Brief,
    RedactionReport,
    Release,
}

impl ArtifactKind {
    pub fn store(self) -> vpt_domain::layout::StoreKey {
        use vpt_domain::layout::StoreKey;
        match self {
            Self::Audio => StoreKey::Audio,
            Self::EngineOutput => StoreKey::EngineOutputs,
            Self::Transcript => StoreKey::Transcripts,
            Self::Analysis => StoreKey::Analysis,
            Self::Brief => StoreKey::Briefs,
            Self::RedactionReport => StoreKey::Drafts,
            Self::Release => StoreKey::Released,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::EngineOutput => "engine_output",
            Self::Transcript => "transcript",
            Self::Analysis => "analysis",
            Self::Brief => "brief",
            Self::RedactionReport => "redaction_report",
            Self::Release => "release",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "audio" => Some(Self::Audio),
            "engine_output" => Some(Self::EngineOutput),
            "transcript" => Some(Self::Transcript),
            "analysis" => Some(Self::Analysis),
            "brief" => Some(Self::Brief),
            "redaction_report" => Some(Self::RedactionReport),
            "release" => Some(Self::Release),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIntent {
    pub kind: ArtifactKind,
    pub owner: ArtifactOwner,
    pub path: PathBuf,
    pub expected: Sha256Digest,
    pub recorded_at: UtcInstant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionIntent {
    pub id: i64,
    pub kind: ArtifactKind,
    pub owner: ArtifactOwner,
    pub path: PathBuf,
    pub expected: Sha256Digest,
    pub recorded_at: UtcInstant,
}

pub trait RetentionJournal {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError>;
    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError>;
    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError>;
}

#[cfg(test)]
mod tests;
