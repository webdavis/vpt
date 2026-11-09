use super::{LedgerError, OccasionRecord};
use std::path::{Path, PathBuf};
use vpt_domain::{
    brief::document::Snapshot,
    digest::Sha256Digest,
    occasion::OccasionId,
    time::{UtcInstant, UtcOffset},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BriefVersion {
    pub content: Sha256Digest,
    pub snapshot: Sha256Digest,
}

#[derive(Clone, PartialEq)]
pub struct BriefArtifact {
    pub snapshot: Snapshot,
    pub target: PathBuf,
    pub version: BriefVersion,
    pub text: String,
    pub managed_tags: Vec<String>,
    pub first_written: UtcInstant,
    pub write_offset: UtcOffset,
    pub published_at: Option<UtcInstant>,
}
impl std::fmt::Debug for BriefArtifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BriefArtifact")
            .field("occasion", &self.snapshot.id)
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BriefChange {
    pub expected_occasion: OccasionRecord,
    pub expected: Option<BriefVersion>,
    pub artifact: BriefArtifact,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BriefPublication {
    pub occasion: OccasionId,
    pub expected: BriefVersion,
    pub at: UtcInstant,
}
pub trait BriefLedger {
    fn brief(&self, occasion: &OccasionId) -> Result<Option<BriefArtifact>, LedgerError>;
    fn brief_at(&self, target: &Path) -> Result<Option<BriefArtifact>, LedgerError>;
}
