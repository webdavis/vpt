use super::{Grounder, GroundingError, MAX_ITEMS};
use crate::proposal::{CheckedClaim, CheckedProposal, Claim, Proposal};

impl Grounder<'_> {
    pub fn proposal(&mut self, proposal: Proposal) -> Result<CheckedProposal, GroundingError> {
        if proposal.summary.len() > MAX_ITEMS
            || proposal.actions.len() > MAX_ITEMS
            || proposal.tags.len() > 256
            || proposal.relations.len() > 256
        {
            return Err(GroundingError::CapacityExceeded);
        }
        let mut check = |claim: Claim| {
            let grounding = self.check(&claim)?;
            Ok(CheckedClaim { claim, grounding })
        };
        let summary = proposal
            .summary
            .into_iter()
            .map(&mut check)
            .collect::<Result<_, _>>()?;
        let actions = proposal
            .actions
            .into_iter()
            .map(&mut check)
            .collect::<Result<_, _>>()?;
        Ok(CheckedProposal {
            by: proposal.by,
            summary,
            actions,
            tags: proposal.tags,
            relations: proposal.relations,
        })
    }
}
