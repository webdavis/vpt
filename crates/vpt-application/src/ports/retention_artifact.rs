use super::{ArtifactKind, LedgerError};
use std::path::PathBuf;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, occasion::OccasionId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactOwner {
    Recording(RecordingId),
    Occasion(OccasionId),
}

impl ArtifactOwner {
    pub fn parse(kind: ArtifactKind, text: &str) -> Option<Self> {
        if kind == ArtifactKind::Brief {
            OccasionId::parse(text).ok().map(Self::Occasion)
        } else {
            RecordingId::parse(text).ok().map(Self::Recording)
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Recording(id) => id.as_str(),
            Self::Occasion(id) => id.as_str(),
        }
    }
    pub fn recording(&self) -> Option<&RecordingId> {
        match self {
            Self::Recording(id) => Some(id),
            Self::Occasion(_) => None,
        }
    }
    pub fn matches_kind(&self, kind: ArtifactKind) -> bool {
        matches!((self, kind), (Self::Occasion(_), ArtifactKind::Brief))
            || matches!(self, Self::Recording(_)) && kind != ArtifactKind::Brief
    }
}

impl From<RecordingId> for ArtifactOwner {
    fn from(id: RecordingId) -> Self {
        Self::Recording(id)
    }
}
impl From<OccasionId> for ArtifactOwner {
    fn from(id: OccasionId) -> Self {
        Self::Occasion(id)
    }
}
impl std::fmt::Display for ArtifactOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionStatus {
    Published,
    PublicationPending,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionArtifact {
    pub kind: ArtifactKind,
    pub owner: ArtifactOwner,
    pub path: PathBuf,
    pub digest: Sha256Digest,
    pub status: RetentionStatus,
    pub trashed_at: Option<vpt_domain::time::UtcInstant>,
}

pub trait RetentionArtifacts {
    /// Metadata only, in path order, bounded to 65,536 artifacts and 16 MiB of metadata.
    fn retention_artifacts(&self) -> Result<Vec<RetentionArtifact>, LedgerError>;
}

impl RetentionArtifact {
    pub fn matches_intent(&self, intent: &super::RetentionIntent) -> bool {
        // Notes may contain operator prose: the intent hashes validated current bytes,
        // while the ledger digest identifies the saved artifact version.
        self.path == intent.path
            && self.kind == intent.kind
            && self.owner == intent.owner
            && self.status != RetentionStatus::PublicationPending
            && (matches!(
                self.kind,
                ArtifactKind::Transcript | ArtifactKind::Analysis | ArtifactKind::Brief
            ) || self.digest == intent.expected)
    }
}
