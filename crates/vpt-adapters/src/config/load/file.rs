use super::{ConfigError, load_text, privacy};
use crate::{Access, ContainedError, RootDir};
use std::{io::Read, path::Path};
use toml::Table;

pub fn load_file(path: &Path) -> Result<Table, ConfigError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|error| config_io(path, error))?;
    let root = RootDir::open(&parent).map_err(|error| config_contained(path, error))?;
    let name = path
        .file_name()
        .ok_or_else(|| config_io(path, std::io::ErrorKind::InvalidInput.into()))?;
    let mut file = root
        .open_file(Path::new(name), Access::Read)
        .map_err(|error| config_contained(path, error))?;
    privacy::validate(&root, &file, path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| config_io(path, error))?;
    load_text(&text)
}

pub(super) fn config_io(path: &Path, error: std::io::Error) -> ConfigError {
    if error.kind() == std::io::ErrorKind::NotFound {
        ConfigError::Missing(path.display().to_string())
    } else {
        ConfigError::Unreadable {
            path: path.display().to_string(),
            detail: error.kind().to_string(),
        }
    }
}

pub(super) fn config_contained(path: &Path, error: ContainedError) -> ConfigError {
    match error {
        ContainedError::Io { kind, .. } => config_io(path, kind.into()),
        _ => ConfigError::PathEscape(path.display().to_string()),
    }
}
