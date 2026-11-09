use super::invalid;
use std::path::{Component, Path};
use vpt_application::ports::LedgerError;

pub(in crate::ledger) fn validate(path: &Path) -> Result<(), LedgerError> {
    if !path.is_absolute()
        || path
            .to_str()
            .is_none_or(|value| value.len() > 65_536 || value.chars().any(char::is_control))
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::RootDir))
        || path.extension().is_none_or(|extension| extension != "md")
    {
        return Err(invalid());
    }
    Ok(())
}
