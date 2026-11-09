use super::*;
use std::collections::BTreeMap;

pub(super) fn inputs(request: &Request<'_>) -> Result<Vec<usize>, BriefError> {
    if !request.occasion.is_valid() {
        return Err(BriefError::InvalidInput);
    }
    if request.candidates.len() > 65_536 || request.terms.len() > 65_536 {
        return Err(BriefError::CapacityExceeded);
    }
    let mut bytes = 64 * 1024 * 1024usize;
    let mut items = 1_048_576usize;
    let mut ids = BTreeMap::new();
    for term in request.terms {
        text(&term.text, &mut bytes)?;
        if term.text.trim().is_empty() {
            return Err(BriefError::InvalidInput);
        }
        if let TermTarget::Unique(path) = &term.target {
            path_text(path, &mut bytes)?;
        }
    }
    for (index, row) in request.candidates.iter().enumerate() {
        candidate(row, &mut bytes, &mut items)?;
        if let Some(old) = ids.insert(row.recording.as_str(), index) {
            if request.candidates[old] != *row {
                return Err(BriefError::ConflictingRecording);
            }
            ids.insert(row.recording.as_str(), old);
        }
    }
    Ok(ids.into_values().collect())
}
fn path_text(path: &std::path::Path, bytes: &mut usize) -> Result<(), BriefError> {
    let value = path.to_str().ok_or(BriefError::InvalidInput)?;
    text(value, bytes)?;
    if !path.is_absolute()
        || value
            .split('/')
            .skip(1)
            .any(|p| matches!(p, "" | "." | ".."))
    {
        return Err(BriefError::InvalidInput);
    }
    Ok(())
}
fn text(value: &str, bytes: &mut usize) -> Result<(), BriefError> {
    if value.len() > 4 * 65_536 || value.chars().count() > 65_536 || value.contains('\0') {
        return Err(BriefError::InvalidInput);
    }
    *bytes = bytes
        .checked_sub(value.len())
        .ok_or(BriefError::CapacityExceeded)?;
    Ok(())
}

fn source_flags(
    source: &TranscriptSource,
    bytes: &mut usize,
    items: &mut usize,
) -> Result<(), BriefError> {
    use crate::review::{FlagContent, Resolution};
    *items = items
        .checked_sub(source.flags.len())
        .ok_or(BriefError::CapacityExceeded)?;
    let has_words = source
        .transcript
        .segments
        .iter()
        .any(|segment| !segment.words.is_empty());
    for flag in &source.flags {
        match &flag.content {
            FlagContent::Lexical(lexical) => {
                *items = items
                    .checked_sub(lexical.ranges.len())
                    .ok_or(BriefError::CapacityExceeded)?;
                if !has_words {
                    return Err(BriefError::InvalidInput);
                }
                text(&lexical.record_text, bytes)?;
                if let Some(value) = &lexical.alternative_text {
                    text(value, bytes)?;
                }
            }
            FlagContent::Verification(finding) => {
                if !finding.valid() {
                    return Err(BriefError::InvalidInput);
                }
                text(&finding.record_text, bytes)?;
            }
            FlagContent::Diagnostic(_) => (),
        }
        if let Resolution::Corrected { text: value, .. } = &flag.resolution {
            text(value, bytes)?;
        }
    }
    Ok(())
}

pub(super) fn candidate(
    row: &Candidate,
    bytes: &mut usize,
    items: &mut usize,
) -> Result<(), BriefError> {
    *items = items
        .checked_sub(row.tags.len() + row.relations.len() + 1)
        .ok_or(BriefError::CapacityExceeded)?;
    if row.tags.len() > 256 {
        return Err(BriefError::CapacityExceeded);
    }
    for tag in &row.tags {
        text(&tag.name, bytes)?;
    }
    for relation in &row.relations {
        text(relation.target.as_str(), bytes)?;
        *items = items
            .checked_sub(relation.ranges.len())
            .ok_or(BriefError::CapacityExceeded)?;
    }
    if let Some(source) = &row.source {
        path_text(&source.note, bytes)?;
        crate::note::validate::inputs(&source.transcript, &source.flags)
            .map_err(|_| BriefError::InvalidInput)?;
        source_flags(source, bytes, items)?;
        for segment in &source.transcript.segments {
            if segment.end > (1u64 << 53) as f64 {
                return Err(BriefError::InvalidInput);
            }
            *items = items
                .checked_sub(segment.words.len() + 1)
                .ok_or(BriefError::CapacityExceeded)?;
            text(&segment.text, bytes)?;
            for word in &segment.words {
                text(&word.text, bytes)?;
            }
        }
    }
    Ok(())
}
