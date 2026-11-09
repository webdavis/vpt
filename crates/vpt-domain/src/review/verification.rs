use crate::{
    digest::Sha256Digest,
    grounding::{Finding, SourceRange},
    note::NoteStage,
};

#[derive(Clone, PartialEq, Eq)]
pub struct VerificationFlag {
    pub artifact: NoteStage,
    pub claim_index: u32,
    pub claim_digest: Sha256Digest,
    /// Claim text with verification annotations stripped; citations remain part of its identity.
    pub record_text: String,
    pub finding: Finding,
    pub sources: Vec<SourceRange>,
}

impl std::fmt::Debug for VerificationFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerificationFlag")
            .field("artifact", &self.artifact)
            .field("claim_index", &self.claim_index)
            .field("claim_digest", &self.claim_digest)
            .field("finding", &self.finding)
            .field("sources", &self.sources)
            .finish()
    }
}

impl VerificationFlag {
    pub fn valid(&self) -> bool {
        self.claim_index < 65_536
            && super::valid_correction(&self.record_text)
            && self.sources.len() <= 256
            && self
                .sources
                .iter()
                .all(|range| range.start_secs < range.end_secs && range.end_secs <= (1u64 << 53))
            && match &self.finding {
                Finding::Unsourced | Finding::BadReference => self.sources.is_empty(),
                Finding::Unsupported(token) => {
                    !self.sources.is_empty() && super::valid_correction(token)
                }
                Finding::BuiltOnFlaggedText => !self.sources.is_empty(),
            }
    }
}
