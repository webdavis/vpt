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
        if let Some(due) = &action.due {
            if !date(&due.date) || due.time.as_deref().is_some_and(|time| !clock(time)) {
                return Err(ProposalDecodeError::Invalid("due date"));
            }
            for source in &due.sources {
                text(source, false, "source reference")?;
            }
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

fn date(value: &str) -> bool {
    let separated =
        value.len() == 10 && value.get(4..5) == Some("-") && value.get(7..8) == Some("-");
    let parts = (
        number(value, 0..4),
        number(value, 5..7),
        number(value, 8..10),
    );
    let (Some(year), Some(month), Some(day)) = parts else {
        return false;
    };
    separated
        && vpt_domain::time::Civil {
            year: i64::from(year),
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
        }
        .is_valid()
}

fn clock(value: &str) -> bool {
    let (Some(hour), Some(minute)) = (number(value, 0..2), number(value, 3..5)) else {
        return false;
    };
    value.len() == 5 && value.get(2..3) == Some(":") && hour < 24 && minute < 60
}

fn number(value: &str, range: std::ops::Range<usize>) -> Option<u32> {
    let digits = value.get(range)?;
    digits
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| digits.parse().ok())?
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
