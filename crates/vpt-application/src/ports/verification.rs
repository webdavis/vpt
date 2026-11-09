use super::{NoteStage};
use vpt_domain::{
    digest::Sha256Digest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedClaims {
    pub artifact: NoteStage,
    pub digest: Sha256Digest,
    pub grounding_window_secs: u32,
}
