use vpt_domain::proposal::{Action, Claim, Due, Owner, Proposal, Related, Tag};
use vpt_protocol::proposal as protocol;
pub use vpt_protocol::proposal::ProposalDecodeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedProposal {
    pub proposal: Proposal,
    pub diagnostics: Vec<String>,
}

pub fn decode(bytes: &[u8]) -> Result<DecodedProposal, ProposalDecodeError> {
    let decoded = vpt_protocol::proposal::decode(bytes)?;
    let document = decoded.document;
    Ok(DecodedProposal {
        diagnostics: decoded.diagnostics,
        proposal: Proposal {
            by: document.by,
            summary: claims(document.summary),
            actions: document.actions.into_iter().map(action).collect(),
            tags: document
                .tags
                .into_iter()
                .map(|tag| Tag {
                    tag: tag.tag,
                    reason: tag.reason,
                })
                .collect(),
            relations: document
                .relations
                .into_iter()
                .map(|relation| Related {
                    target: relation.target,
                    reason: relation.reason,
                })
                .collect(),
        },
    })
}

fn action(action: protocol::Action) -> Action {
    Action {
        claim: Claim {
            text: action.text,
            source_references: action.sources,
        },
        owner: action.owner.map(|owner| match owner {
            protocol::Owner::Me => Owner::Me,
            protocol::Owner::Other(name) => Owner::Other(name),
        }),
        due: action.due.map(|due| Due {
            date: due.date,
            time: due.time,
            source_references: due.sources,
        }),
    }
}

fn claims(claims: Vec<protocol::Claim>) -> Vec<Claim> {
    claims
        .into_iter()
        .map(|claim| Claim {
            text: claim.text,
            source_references: claim.sources,
        })
        .collect()
}

#[cfg(test)]
mod tests;
