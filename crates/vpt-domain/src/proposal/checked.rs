use super::{Claim, Related, Tag};
use crate::grounding::GroundedClaim;

#[derive(Clone, PartialEq, Eq)]
pub struct CheckedProposal {
    pub by: String,
    pub summary: Vec<CheckedClaim>,
    pub actions: Vec<CheckedClaim>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Related>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedClaim {
    pub claim: Claim,
    pub grounding: GroundedClaim,
}

impl std::fmt::Debug for CheckedProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckedProposal")
            .field("summary", &self.summary.len())
            .field("actions", &self.actions.len())
            .field("tags", &self.tags.len())
            .field("relations", &self.relations.len())
            .finish()
    }
}
