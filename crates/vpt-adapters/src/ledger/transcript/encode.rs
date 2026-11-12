use super::corrupt;
use serde::Serialize;
use vpt_application::ports::LedgerError;
use vpt_domain::transcript::Transcript;

pub(super) fn check_shape(transcript: &Transcript) -> Result<(), LedgerError> {
    let mut remaining = 16_777_216usize;
    let mut text = |value: &str| {
        if value.len() > 4 * 65_536 || value.chars().count() > 65_536 {
            return Err(corrupt());
        }
        remaining = remaining.checked_sub(value.len()).ok_or_else(corrupt)?;
        Ok(())
    };
    for value in [
        &transcript.language,
        &transcript.engine.name,
        &transcript.engine.tool,
        &transcript.engine.version,
        &transcript.engine.model,
        &transcript.engine.family,
    ] {
        text(value)?;
    }
    if transcript.segments.len() > 65_536 {
        return Err(corrupt());
    }
    let mut words = 0usize;
    for segment in &transcript.segments {
        text(&segment.text)?;
        if let Some(speaker) = &segment.speaker {
            text(speaker)?;
        }
        if segment.words.len() > 65_536 {
            return Err(corrupt());
        }
        words = words.checked_add(segment.words.len()).ok_or_else(corrupt)?;
        for word in &segment.words {
            text(&word.text)?;
        }
    }
    // Every serialized word and segment needs at least twenty structural bytes.
    let objects = words
        .checked_add(transcript.segments.len())
        .ok_or_else(corrupt)?;
    if objects > remaining / 20 {
        return Err(corrupt());
    }
    Ok(())
}

pub(super) fn json(value: &impl Serialize, maximum: usize) -> Result<String, LedgerError> {
    super::super::json::encode(value, maximum).map_err(|_| corrupt())
}
