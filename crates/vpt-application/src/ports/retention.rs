use super::ledger::LedgerError;
use std::path::PathBuf;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Audio,
}

impl ArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        (text == "audio").then_some(Self::Audio)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIntent {
    pub kind: ArtifactKind,
    pub recording: RecordingId,
    pub path: PathBuf,
    pub expected: Sha256Digest,
    pub recorded_at: UtcInstant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionIntent {
    pub id: i64,
    pub kind: ArtifactKind,
    pub recording: RecordingId,
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
mod tests {
    use super::ArtifactKind;

    #[test]
    fn artifact_kind_accepts_only_audio() {
        assert_eq!(ArtifactKind::Audio.as_str(), "audio");
        assert_eq!(ArtifactKind::parse("audio"), Some(ArtifactKind::Audio));
        assert_eq!(ArtifactKind::parse("Audio"), None);
        assert_eq!(ArtifactKind::parse("note"), None);
    }
}
