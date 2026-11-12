use super::*;
use crate::{
    note::{MAX_ITEMS, MAX_SOURCE},
    proposal::Owner,
};

pub(super) fn input(proposal: &CheckedProposal) -> Result<(), RenderError> {
    if proposal.summary.len() > MAX_ITEMS || proposal.actions.len() > MAX_ITEMS {
        return Err(RenderError::CapacityExceeded);
    }
    let mut total = 0;
    for action in &proposal.actions {
        if let Some(Owner::Other(Some(name))) = &action.owner {
            text(name, &mut total)?;
            if name.trim().is_empty() {
                return Err(RenderError::InvalidProposal);
            }
        }
    }
    for checked in proposal.claims() {
        let claim = &checked.claim;
        text(&claim.text, &mut total)?;
        if claim.text.trim().is_empty() {
            return Err(RenderError::InvalidProposal);
        }
        if claim.source_references.len() > 256
            || checked.grounding.sources.len() > 256
            || checked.grounding.inherited_flags.len() > MAX_ITEMS
        {
            return Err(RenderError::CapacityExceeded);
        }
        for source in &claim.source_references {
            text(source, &mut total)?;
        }
        if let Some(Finding::Unsupported(token)) = &checked.grounding.finding {
            text(token, &mut total)?;
        }
    }
    Ok(())
}

fn text(text: &str, total: &mut usize) -> Result<(), RenderError> {
    if text.len() > MAX_ITEMS * 4 || text.chars().count() > MAX_ITEMS {
        return Err(RenderError::CapacityExceeded);
    }
    *total += text.len();
    if *total > MAX_SOURCE {
        return Err(RenderError::CapacityExceeded);
    }
    Ok(())
}
