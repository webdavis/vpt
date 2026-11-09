use super::{digest, invalid};
use serde::{Deserialize, Serialize};
use std::path::Component;
use vpt_application::ports::{DirtyPublication, LedgerError};
use vpt_domain::time::UtcInstant;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Saved {
    target: String,
    expected_previous: Option<String>,
    intended: String,
}

pub(super) fn save(entry: &DirtyPublication, at: UtcInstant) -> Result<Saved, LedgerError> {
    check(entry, at)?;
    Ok(Saved {
        target: entry.target.to_str().ok_or_else(invalid)?.into(),
        expected_previous: entry.expected_previous.map(|value| value.hex()),
        intended: entry.intended.hex(),
    })
}

pub(super) fn restore(saved: Saved, at: UtcInstant) -> Result<DirtyPublication, LedgerError> {
    let entry = DirtyPublication {
        target: saved.target.into(),
        expected_previous: saved.expected_previous.as_deref().map(digest).transpose()?,
        intended: digest(&saved.intended)?,
        recorded_at: at,
    };
    check(&entry, at)?;
    Ok(entry)
}

fn check(entry: &DirtyPublication, at: UtcInstant) -> Result<(), LedgerError> {
    if entry.recorded_at != at
        || entry.expected_previous == Some(entry.intended)
        || !entry.target.is_absolute()
        || entry.target.file_name().is_none()
        || entry
            .target
            .to_str()
            .is_none_or(|path| path.len() > 65_536 || path.chars().any(char::is_control))
        || entry
            .target
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(invalid());
    }
    Ok(())
}
