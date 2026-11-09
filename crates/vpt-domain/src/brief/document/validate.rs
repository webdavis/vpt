mod counts;
mod tasks;
use super::*;
use crate::brief::{ContextStatus, TermState};
use std::collections::BTreeMap;

pub(super) fn validate(document: &Document<'_>) -> Result<(), DocumentError> {
    if !document.occasion.is_valid() {
        return Err(DocumentError::Invalid);
    }
    let pack = document.pack;
    let selection = &pack.selection;
    if pack.items.len() > 65_536
        || pack.sources.len() > 65_536
        || selection.decisions.len() > 65_536
        || selection.selected.len() > 65_536
    {
        return Err(DocumentError::CapacityExceeded);
    }
    counts::check(pack)?;
    if selection.participants.len() != document.occasion.participants.len() {
        return Err(DocumentError::Invalid);
    }
    let mut bytes = 16 * 1024 * 1024;
    tasks::check(document, &mut bytes)?;
    text(&document.occasion.title, &mut bytes)?;
    for participant in &document.occasion.participants {
        text(&participant.name, &mut bytes)?;
        if let Some(email) = &participant.email {
            text(email, &mut bytes)?;
        }
    }
    for (index, matched) in selection.participants.iter().enumerate() {
        if matched.participant != index
            || (matched.term == TermState::Confirmed) != matched.matched_note.is_some()
            || (matched.term == TermState::Unconfirmed) != matched.confirmation.is_some()
        {
            return Err(DocumentError::Invalid);
        }
        if let Some(path) = &matched.matched_note {
            text(
                super::source_path(path).ok_or(DocumentError::Invalid)?,
                &mut bytes,
            )?;
        }
        if let Some(command) = &matched.confirmation {
            text(command, &mut bytes)?;
        }
    }
    let mut sources = BTreeMap::new();
    for source in &pack.sources {
        text(
            super::source_path(&source.note).ok_or(DocumentError::Invalid)?,
            &mut bytes,
        )?;
        if sources.insert(source.recording.as_str(), source).is_some() {
            return Err(DocumentError::Invalid);
        }
    }
    let mut selected = BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    for decision in &selection.decisions {
        if !seen.insert(decision.recording.as_str())
            || decision.matched.is_some() == decision.rejected.is_some()
            || (decision.selected && decision.matched.is_none())
        {
            return Err(DocumentError::Invalid);
        }
        if let Some(matched) = &decision.matched {
            if matched.values.len() > 65_536 || matched.anchors.len() > 65_536 {
                return Err(DocumentError::CapacityExceeded);
            }
            for value in &matched.values {
                text(value, &mut bytes)?;
            }
            if decision.selected {
                selected.insert(decision.recording.as_str(), matched.selector);
            }
        }
    }
    if selected.len() != sources.len()
        || selected.len() != selection.selected.len()
        || !selected.keys().eq(sources.keys())
    {
        return Err(DocumentError::Invalid);
    }
    for quote in &pack.items {
        let source = sources
            .get(quote.source.recording.as_str())
            .ok_or(DocumentError::Invalid)?;
        if source.digest != quote.source.digest
            || source.note != quote.source.note
            || selected[quote.source.recording.as_str()] != quote.selector
        {
            return Err(DocumentError::Invalid);
        }
        text(&quote.text, &mut bytes)?;
        text(
            super::source_path(&quote.source.note).ok_or(DocumentError::Invalid)?,
            &mut bytes,
        )?;
    }
    for half in [&document.context.calendar, &document.context.tasks] {
        if let ContextStatus::Failed { detail } = half {
            if detail.trim().is_empty() {
                return Err(DocumentError::Invalid);
            }
            if detail.chars().count() > 1024 {
                return Err(DocumentError::CapacityExceeded);
            }
            text(detail, &mut bytes)?;
        }
    }
    Ok(())
}
fn text(value: &str, remaining: &mut usize) -> Result<(), DocumentError> {
    if value.len() > 4 * 65_536 || value.chars().count() > 65_536 || value.contains('\0') {
        return Err(DocumentError::Invalid);
    }
    *remaining = remaining
        .checked_sub(value.len())
        .ok_or(DocumentError::CapacityExceeded)?;
    Ok(())
}
