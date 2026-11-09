use super::LedgerError;
use vpt_domain::{
    digest::Sha256Digest, identity::RecordingId, proposal::CheckedProposal, time::UtcInstant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundingBasis {
    pub transcript: Sha256Digest,
    pub lexical_flags: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedProposal {
    pub vocabulary_publication: Option<super::DirtyPublication>,
    pub recording: RecordingId,
    pub input_digest: Sha256Digest,
    pub basis: GroundingBasis,
    pub grounding_window_secs: u32,
    pub proposal: CheckedProposal,
    pub accepted_at: UtcInstant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalChange {
    Accept(Box<AcceptedProposal>),
    Fail {
        recording: RecordingId,
        at: UtcInstant,
    },
}

impl ProposalChange {
    pub fn recording(&self) -> &RecordingId {
        match self {
            Self::Accept(accepted) => &accepted.recording,
            Self::Fail { recording, .. } => recording,
        }
    }
    pub fn at(&self) -> UtcInstant {
        match self {
            Self::Accept(accepted) => accepted.accepted_at,
            Self::Fail { at, .. } => *at,
        }
    }
}

pub trait ProposalLedger {
    fn accepted_proposal(&self, id: &RecordingId) -> Result<Option<AcceptedProposal>, LedgerError>;
    fn grounding_basis(&self, id: &RecordingId) -> Result<Option<GroundingBasis>, LedgerError>;
}
