use super::{Claim, Due, Owner, Related, Tag};
use crate::grounding::{Finding, GroundedClaim};

#[derive(Clone, PartialEq, Eq)]
pub struct CheckedProposal {
    pub by: String,
    pub summary: Vec<CheckedClaim>,
    pub actions: Vec<CheckedAction>,
    pub tags: Vec<Tag>,
    pub relations: Vec<Related>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedClaim {
    pub claim: Claim,
    pub grounding: GroundedClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedAction {
    pub claim: CheckedClaim,
    pub owner: Option<Owner>,
    pub due: Option<CheckedDue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedDue {
    pub due: Due,
    pub grounding: GroundedClaim,
}

impl CheckedDue {
    pub fn withheld(&self) -> bool {
        matches!(
            self.grounding.finding,
            Some(Finding::Unsourced | Finding::BadReference)
        )
    }
}

impl CheckedProposal {
    pub fn claims(&self) -> impl Iterator<Item = &CheckedClaim> {
        self.summary
            .iter()
            .chain(self.actions.iter().map(|action| &action.claim))
    }
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
