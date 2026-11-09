use super::*;
use vpt_protocol::proposal::{Claim, ProposalDocument, Relation, Tag};

pub(super) fn encode(proposal: &CheckedProposal) -> Result<String, LedgerError> {
    let mut remaining = 1_048_576usize;
    let mut text = |value: &str| {
        remaining = remaining.checked_sub(value.len() + 1).ok_or_else(invalid)?;
        Ok::<_, LedgerError>(())
    };
    if proposal.summary.len() > 65_536
        || proposal.actions.len() > 65_536
        || proposal.tags.len() > 256
        || proposal.relations.len() > 256
    {
        return Err(invalid());
    }
    text(&proposal.by)?;
    for checked in proposal.summary.iter().chain(&proposal.actions) {
        text(&checked.claim.text)?;
        if checked.claim.source_references.len() > 256 {
            return Err(invalid());
        }
        for source in &checked.claim.source_references {
            text(source)?;
        }
    }
    for tag in &proposal.tags {
        text(&tag.tag)?;
        text(&tag.reason)?;
    }
    for relation in &proposal.relations {
        text(&relation.target)?;
        text(&relation.reason)?;
    }
    let document = ProposalDocument {
        schema: "vpt.proposal/1".into(),
        by: proposal.by.clone(),
        summary: claims(&proposal.summary),
        actions: claims(&proposal.actions),
        tags: proposal
            .tags
            .iter()
            .map(|tag| Tag {
                tag: tag.tag.clone(),
                reason: tag.reason.clone(),
            })
            .collect(),
        relations: proposal
            .relations
            .iter()
            .map(|relation| Relation {
                kind: "related".into(),
                target: relation.target.clone(),
                reason: relation.reason.clone(),
            })
            .collect(),
    };
    let json = crate::ledger::json::encode(&document, 1_048_576).map_err(|_| invalid())?;
    vpt_protocol::proposal::decode(json.as_bytes()).map_err(|_| invalid())?;
    Ok(json)
}

fn claims(claims: &[CheckedClaim]) -> Vec<Claim> {
    claims
        .iter()
        .map(|checked| Claim {
            text: checked.claim.text.clone(),
            sources: checked.claim.source_references.clone(),
        })
        .collect()
}
