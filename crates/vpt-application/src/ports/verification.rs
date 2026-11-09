use super::{GroundingBasis, NoteStage};
use vpt_domain::{
    digest::Sha256Digest, identity::RecordingId, review::VerificationFlag, time::UtcInstant,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationChange {
    pub grounding_window_secs: u32,
    /// A current disk snapshot when claims differ from the saved note; publication must replace it.
    pub observed: Option<super::StoredNote>,
    pub recording: RecordingId,
    pub artifact: NoteStage,
    /// Digest of the checked content region, from `observed` when present or saved state otherwise.
    pub input_digest: Sha256Digest,
    pub basis: GroundingBasis,
    pub findings: Vec<VerificationFlag>,
    pub at: UtcInstant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedClaims {
    pub artifact: NoteStage,
    pub digest: Sha256Digest,
    pub grounding_window_secs: u32,
}
