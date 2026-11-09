use super::*;
use crate::normalize::{TermMatcher, canonical};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) type Targets = BTreeMap<PathBuf, BTreeSet<String>>;
pub(super) struct Prepared {
    pub participants: Vec<ParticipantMatch>,
    pub participant_targets: Targets,
    pub title_targets: Targets,
}

pub(super) fn prepare(request: &Request<'_>) -> Result<Prepared, BriefError> {
    let mut known = BTreeMap::new();
    let mut remaining = 16 * 1024 * 1024;
    for term in request.terms {
        let key = canonical(&term.text)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if key.len() > 4 * 65_536 || key.chars().count() > 65_536 {
            return Err(BriefError::CapacityExceeded);
        }
        super::direct::spend(&mut remaining, key.len() + 1)?;
        if let Some(old) = known.insert(key.clone(), term)
            && old.target != term.target
        {
            return Err(BriefError::InvalidInput);
        }
    }
    let mut result = Prepared {
        participants: vec![],
        participant_targets: Targets::new(),
        title_targets: Targets::new(),
    };
    for (participant, value) in request.occasion.participants.iter().enumerate() {
        let name = if value.name.trim().is_empty() {
            value.email.as_deref().unwrap_or_default()
        } else {
            &value.name
        };
        let key = canonical(name)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let (term, matched_note) = match known.get(&key).map(|term| &term.target) {
            None => (TermState::Unconfirmed, None),
            Some(TermTarget::Missing) => (TermState::MissingNote, None),
            Some(TermTarget::Ambiguous) => (TermState::AmbiguousNote, None),
            Some(TermTarget::Unique(path)) => {
                result
                    .participant_targets
                    .entry(path.clone())
                    .or_default()
                    .insert(key);
                (TermState::Confirmed, Some(path.clone()))
            }
        };
        result.participants.push(ParticipantMatch {
            participant,
            term,
            matched_note,
            confirmation: (term == TermState::Unconfirmed)
                .then(|| format!("vpt confirm --term '{}'", name.replace('\'', "'\\''"))),
        });
    }
    let names: Vec<_> = known.keys().map(String::as_str).collect();
    let title: Vec<_> = request.occasion.title.split_whitespace().collect();
    let mut matches = BTreeSet::new();
    TermMatcher::new(&names)
        .map_err(|_| BriefError::CapacityExceeded)?
        .scan(&title, |index, _| {
            matches.insert(index);
            Ok(())
        })
        .map_err(|_| BriefError::CapacityExceeded)?;
    for index in matches {
        let key = names[index];
        if let TermTarget::Unique(path) = &known[key].target {
            result
                .title_targets
                .entry(path.clone())
                .or_default()
                .insert(key.into());
        }
    }
    Ok(result)
}
