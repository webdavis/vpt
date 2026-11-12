use super::*;
use vpt_domain::proposal::{CheckedAction, CheckedClaim, Owner};
use vpt_protocol::proposal::{self as protocol, Claim, ProposalDocument, Relation, Tag};

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
    for action in &proposal.actions {
        if let Some(Owner::Other(Some(name))) = &action.owner {
            text(name)?;
        }
        if let Some(checked) = &action.due {
            let due = &checked.due;
            text(&due.date)?;
            text(due.time.as_deref().unwrap_or_default())?;
            if due.source_references.len() > 256 {
                return Err(invalid());
            }
            for source in &due.source_references {
                text(source)?;
            }
        }
    }
    for checked in proposal.claims() {
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
        summary: proposal.summary.iter().map(claim).collect(),
        actions: proposal.actions.iter().map(action).collect(),
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

fn claim(checked: &CheckedClaim) -> Claim {
    Claim {
        text: checked.claim.text.clone(),
        sources: checked.claim.source_references.clone(),
    }
}

fn action(action: &CheckedAction) -> protocol::Action {
    let Claim { text, sources } = claim(&action.claim);
    protocol::Action {
        text,
        sources,
        owner: action.owner.as_ref().map(|owner| match owner {
            Owner::Me => protocol::Owner::Me,
            Owner::Other(name) => protocol::Owner::Other(name.clone()),
        }),
        due: action.due.as_ref().map(|checked| protocol::Due {
            date: checked.due.date.clone(),
            time: checked.due.time.clone(),
            sources: checked.due.source_references.clone(),
        }),
    }
}
