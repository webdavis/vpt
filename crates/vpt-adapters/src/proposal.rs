use vpt_domain::proposal::{Claim, Proposal, Related, Tag};
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
            actions: claims(document.actions),
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

fn claims(claims: Vec<vpt_protocol::proposal::Claim>) -> Vec<Claim> {
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
