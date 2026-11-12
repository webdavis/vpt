use super::{Grounder, GroundingError, MAX_ITEMS};
use crate::proposal::{Action, CheckedAction, CheckedClaim, CheckedProposal, Claim, Proposal};

impl Grounder<'_> {
    pub fn proposal(&mut self, proposal: Proposal) -> Result<CheckedProposal, GroundingError> {
        if proposal.summary.len() > MAX_ITEMS
            || proposal.actions.len() > MAX_ITEMS
            || proposal.tags.len() > 256
            || proposal.relations.len() > 256
        {
            return Err(GroundingError::CapacityExceeded);
        }
        let summary = proposal
            .summary
            .into_iter()
            .map(|claim| self.claim(claim))
            .collect::<Result<_, _>>()?;
        let actions = proposal
            .actions
            .into_iter()
            .map(|action| self.action(action))
            .collect::<Result<_, _>>()?;
        Ok(CheckedProposal {
            by: proposal.by,
            summary,
            actions,
            tags: proposal.tags,
            relations: proposal.relations,
        })
    }

    fn claim(&mut self, claim: Claim) -> Result<CheckedClaim, GroundingError> {
        let grounding = self.check(&claim)?;
        Ok(CheckedClaim { claim, grounding })
    }

    fn action(&mut self, action: Action) -> Result<CheckedAction, GroundingError> {
        Ok(CheckedAction {
            claim: self.claim(action.claim)?,
            owner: action.owner,
        })
    }
}
