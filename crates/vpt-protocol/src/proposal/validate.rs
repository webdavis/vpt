use super::{ProposalDecodeError, ProposalDocument};

pub(super) fn validate(document: &ProposalDocument) -> Result<(), ProposalDecodeError> {
    text(&document.by, true, "author")?;
    for claim in document.summary.iter().chain(&document.actions) {
        text(&claim.text, true, "claim text")?;
        for source in &claim.sources {
            text(source, false, "source reference")?;
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
