use super::{RetentionArtifact, StoreError};
use vpt_domain::digest::Sha256Digest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetentionCheck {
    Missing,
    Owned(Sha256Digest),
    Modified,
}

pub trait RetentionFiles {
    /// Inspect only the pinned path. Owned returns the current bytes' digest for the move journal.
    fn check_retention(&self, artifact: &RetentionArtifact) -> Result<RetentionCheck, StoreError>;
}
