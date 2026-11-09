use vpt_domain::{digest::Sha256Digest, identity::RecordingId, review::Resolution};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlagResolutionChange {
    pub recording: RecordingId,
    pub flag: Sha256Digest,
    pub expected: Resolution,
    pub resolution: Resolution,
}
