use super::{RootError, directory};
use std::path::PathBuf;
use vpt_application::{NoteProfile, Settings};

pub fn resolve_vault(settings: &Settings) -> Result<Option<PathBuf>, RootError> {
    if settings.note.profile != NoteProfile::Obsidian {
        return Ok(None);
    }
    let key = "note.obsidian.vault_root";
    let path = settings
        .note
        .obsidian
        .vault_root
        .as_deref()
        .filter(|path| path.is_absolute())
        .ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    directory::existing_directory(path, key)?
        .ok_or_else(|| RootError::NotADirectory { key: key.into() })
        .map(Some)
}
