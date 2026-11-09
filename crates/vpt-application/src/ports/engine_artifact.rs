use super::LedgerError;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, time::UtcInstant};

#[derive(Clone, PartialEq)]
pub struct EngineArtifact {
    pub recording: RecordingId,
    pub engine: String,
    pub target: PathBuf,
    pub digest: Sha256Digest,
    pub duration: f64,
    pub language_source: vpt_domain::transcript::LanguageSource,
    pub bytes: Vec<u8>,
    pub created_at: UtcInstant,
}

impl fmt::Debug for EngineArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineArtifact")
            .field("recording", &self.recording)
            .field("engine", &self.engine)
            .field("target", &self.target)
            .field("language_source", &self.language_source)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

pub trait EngineOutputLedger {
    fn engine_output(&self, target: &Path) -> Result<Option<EngineArtifact>, LedgerError>;
}
