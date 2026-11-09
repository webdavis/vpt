use super::{MAX_BYTES, MAX_ITEMS, conflict, invalid, ranges};
use std::collections::HashSet;
use vpt_application::ports::{AmbiguityChange, LedgerError};
use vpt_domain::relations::MentionAmbiguity;

pub(crate) fn ordered(
    mut values: Vec<MentionAmbiguity>,
) -> Result<Vec<MentionAmbiguity>, LedgerError> {
    validate(&values)?;
    values.sort_by(|a, b| a.term.cmp(&b.term));
    Ok(values)
}

pub(crate) fn prepare(
    change: Option<&AmbiguityChange>,
    current: &[MentionAmbiguity],
) -> Result<Option<Vec<MentionAmbiguity>>, LedgerError> {
    let Some(change) = change else {
        return Ok(None);
    };
    validate(&change.expected)?;
    validate(&change.ambiguities)?;
    if ordered(change.expected.clone())? != current {
        return Err(conflict());
    }
    let values = ordered(change.ambiguities.clone())?;
    encode(&values)?;
    Ok(Some(values))
}

pub(crate) fn encode(values: &[MentionAmbiguity]) -> Result<Vec<(String, String)>, LedgerError> {
    validate(values)?;
    let mut remaining = MAX_BYTES;
    let mut rows = Vec::new();
    for value in values {
        remaining = remaining
            .checked_sub(value.term.len())
            .ok_or_else(invalid)?;
        let ranges = ranges::encode(&value.ranges, remaining)?;
        remaining = remaining.checked_sub(ranges.len()).ok_or_else(invalid)?;
        rows.push((value.term.clone(), ranges));
    }
    Ok(rows)
}

fn validate(values: &[MentionAmbiguity]) -> Result<(), LedgerError> {
    if values.len() > MAX_ITEMS {
        return Err(invalid());
    }
    let mut remaining = MAX_BYTES;
    let mut normalized = MAX_BYTES;
    let mut range_count = MAX_ITEMS;
    let mut keys = HashSet::new();
    for value in values {
        let term = &value.term;
        if term.len() > 4 * MAX_ITEMS
            || term.chars().count() > MAX_ITEMS
            || term.chars().any(|ch| ch.is_control() && ch != '\t')
            || value.ranges.is_empty()
        {
            return Err(invalid());
        }
        remaining = remaining.checked_sub(term.len()).ok_or_else(invalid)?;
        let canonical = vpt_domain::normalize::canonical(term);
        if canonical.len() > 4 * MAX_ITEMS || canonical.chars().count() > MAX_ITEMS {
            return Err(invalid());
        }
        normalized = normalized
            .checked_sub(canonical.len())
            .ok_or_else(invalid)?;
        let key = canonical.split_whitespace().collect::<Vec<_>>().join(" ");
        if key.is_empty() || !keys.insert(key) {
            return Err(invalid());
        }
        ranges::validate(&value.ranges, &mut range_count)?;
    }
    Ok(())
}
