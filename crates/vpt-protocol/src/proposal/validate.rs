use super::{Owner, ProposalDecodeError, ProposalDocument};

pub(super) fn validate(document: &ProposalDocument) -> Result<(), ProposalDecodeError> {
    text(&document.by, true, "author")?;
    let claims = document
        .summary
        .iter()
        .map(|claim| (&claim.text, &claim.sources));
    let actions = document
        .actions
        .iter()
        .map(|action| (&action.text, &action.sources));
    for (claim, sources) in claims.chain(actions) {
        text(claim, true, "claim text")?;
        for source in sources {
            text(source, false, "source reference")?;
        }
    }
    for action in &document.actions {
        if let Some(Owner::Other(Some(name))) = &action.owner {
            text(name, true, "owner")?;
        }
    }
    for tag in &document.tags {
        text(&tag.tag, false, "tag")?;
        text(&tag.reason, false, "tag reason")?;
    }
    for relation in &document.relations {
        if relation.kind != "related" {
            return Err(ProposalDecodeError::Invalid("relation kind"));
        }
        text(&relation.target, true, "relation target")?;
        text(&relation.reason, false, "relation reason")?;
    }
    Ok(())
}

fn text(value: &str, required: bool, field: &'static str) -> Result<(), ProposalDecodeError> {
    if (required && value.trim().is_empty())
        || value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(ProposalDecodeError::Invalid(field));
    }
    Ok(())
}
