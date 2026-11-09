use super::{
    terms::{Prepared, Targets},
    *,
};
use crate::{
    relations::{Kind, State, Target},
    tags,
};
use std::collections::BTreeSet;

pub(super) fn matched(
    row: &Candidate,
    terms: &Prepared,
    tags: &BTreeSet<String>,
    remaining: &mut usize,
) -> Result<Option<Match>, BriefError> {
    if row.source.is_none() {
        return Ok(None);
    }
    for (selector, targets) in [
        (Selector::Participant, &terms.participant_targets),
        (Selector::Term, &terms.title_targets),
    ] {
        let values = mentions(row, targets, remaining)?;
        if !values.is_empty() {
            return Ok(Some(Match {
                selector,
                values,
                anchors: vec![],
            }));
        }
    }
    let mut values = BTreeSet::new();
    for tag in &row.tags {
        if tag.state == tags::State::Confirmed
            && tags.contains(&tags::key(&tag.name).map_err(|_| BriefError::InvalidInput)?)
        {
            spend(remaining, tag.name.len() + 1)?;
            values.insert(tag.name.clone());
        }
    }
    Ok((!values.is_empty()).then(|| Match {
        selector: Selector::Tag,
        values: values.into_iter().collect(),
        anchors: vec![],
    }))
}

fn mentions(
    row: &Candidate,
    targets: &Targets,
    remaining: &mut usize,
) -> Result<Vec<String>, BriefError> {
    let mut values = BTreeSet::new();
    for relation in &row.relations {
        if relation.kind == Kind::Mentions
            && relation.state == State::Confirmed
            && let Target::Note(path) = &relation.target
            && let Some(terms) = targets.get(std::path::Path::new(path))
        {
            for term in terms {
                spend(remaining, term.len() + 1)?;
                values.insert(term.clone());
            }
        }
    }
    Ok(values.into_iter().collect())
}
pub(super) fn spend(remaining: &mut usize, amount: usize) -> Result<(), BriefError> {
    *remaining = remaining
        .checked_sub(amount)
        .ok_or(BriefError::CapacityExceeded)?;
    Ok(())
}
