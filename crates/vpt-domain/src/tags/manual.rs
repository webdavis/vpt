use super::{NewTags, Options, Provenance, State, Tag, Vocabulary};
use crate::time::UtcInstant;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Confirm,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualError {
    Syntax,
    Capacity,
    ConfirmedLimit,
    InvalidState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualEdit {
    pub tag: Tag,
    pub position: Option<usize>,
}

impl Vocabulary {
    pub fn edit(
        &self,
        current: &[Tag],
        proposal: &str,
        action: Action,
        limit: u32,
        at: UtcInstant,
    ) -> Result<ManualEdit, ManualError> {
        if current.len() > 256 {
            return Err(ManualError::Capacity);
        }
        super::normalize::bounded(proposal).map_err(|_| ManualError::Capacity)?;
        let selected = self
            .select(
                &[proposal.to_owned()],
                Options {
                    new_tags: NewTags::Write,
                    max_per_note: 1,
                    max_suggested: 0,
                },
            )
            .map_err(|_| ManualError::Capacity)?;
        let name = selected.confirmed.first().ok_or(ManualError::Syntax)?;
        let key = super::key(name).map_err(|_| ManualError::Syntax)?;
        let mut seen = HashSet::new();
        let mut position = None;
        let mut confirmed = 0;
        for (index, tag) in current.iter().enumerate() {
            let stored = super::key(&tag.name).map_err(|_| ManualError::InvalidState)?;
            if stored == key {
                position = Some(index);
            }
            if !seen.insert(stored) {
                return Err(ManualError::InvalidState);
            }
            confirmed += u32::from(tag.state == State::Confirmed);
        }
        let previous = position.map(|index| &current[index]);
        if position.is_none() && current.len() == 256 {
            return Err(ManualError::Capacity);
        }
        if action == Action::Confirm
            && confirmed >= limit
            && previous.is_none_or(|tag| tag.state != State::Confirmed)
        {
            return Err(ManualError::ConfirmedLimit);
        }
        let state = match action {
            Action::Confirm => State::Confirmed,
            Action::Reject => State::Rejected,
        };
        let tag = match previous {
            Some(tag) if tag.state == state && tag.provenance == Provenance::Manual => tag.clone(),
            _ => Tag {
                name: previous.map_or(name, |tag| &tag.name).clone(),
                state,
                provenance: Provenance::Manual,
                updated_at: Some(at),
            },
        };
        Ok(ManualEdit { tag, position })
    }
}

#[cfg(test)]
mod tests;
