use std::path::{Path, PathBuf};

use crate::{ContainedError, RootDir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkState {
    Absent,
    LinkTo(PathBuf),
    Other(String),
    Unreadable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymlinkError {
    Occupied(String),
    TargetParentMissing,
    TargetNotDirectory,
    TargetMissing,
    Missing,
    WrongTarget(PathBuf),
    Io(String),
}

pub fn inspect(link: &Path) -> LinkState {
    match std::fs::symlink_metadata(link) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => LinkState::Absent,
        Err(error) => LinkState::Unreadable(error.kind().to_string()),
        Ok(metadata) if metadata.file_type().is_symlink() => std::fs::read_link(link).map_or_else(
            |error| LinkState::Unreadable(error.kind().to_string()),
            LinkState::LinkTo,
        ),
        Ok(metadata) if metadata.is_dir() => LinkState::Other("a directory".into()),
        Ok(_) => LinkState::Other("a file".into()),
    }
}

fn destination(link: &Path, value: &Path) -> PathBuf {
    if value.is_absolute() {
        value.to_path_buf()
    } else {
        link.parent().unwrap_or(Path::new(".")).join(value)
    }
}

fn resolved_directory(path: &Path) -> Result<PathBuf, SymlinkError> {
    let resolved = path.canonicalize().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SymlinkError::TargetMissing,
        std::io::ErrorKind::NotADirectory => SymlinkError::TargetNotDirectory,
        kind => SymlinkError::Io(kind.to_string()),
    })?;
    if !resolved
        .metadata()
        .map_err(|error| SymlinkError::Io(error.kind().to_string()))?
        .is_dir()
    {
        return Err(SymlinkError::TargetNotDirectory);
    }
    Ok(resolved)
}

fn contained(error: ContainedError) -> SymlinkError {
    match error {
        ContainedError::NotADirectory(_) | ContainedError::NotRegular(_) => {
            SymlinkError::TargetNotDirectory
        }
        ContainedError::Io { kind, .. } => SymlinkError::Io(kind.to_string()),
        ContainedError::Escape { .. } => SymlinkError::Io("path escapes checked root".into()),
    }
}

fn ensure_target(target: &Path) -> Result<PathBuf, SymlinkError> {
    match resolved_directory(target) {
        Ok(resolved) => Ok(resolved),
        Err(SymlinkError::TargetMissing) => {
            let parent = target.parent().ok_or(SymlinkError::TargetParentMissing)?;
            let parent = match resolved_directory(parent) {
                Err(SymlinkError::TargetMissing) => {
                    return Err(SymlinkError::TargetParentMissing);
                }
                result => result?,
            };
            let leaf = target
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(SymlinkError::TargetNotDirectory)?;
            let parent = RootDir::open(&parent).map_err(contained)?;
            parent
                .subdirectory(leaf)
                .map(|root| root.path().to_path_buf())
                .map_err(contained)
        }
        Err(error) => Err(error),
    }
}

fn prospective(path: &Path) -> Result<Option<PathBuf>, SymlinkError> {
    match path.canonicalize() {
        Ok(resolved) => Ok(Some(resolved)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let (Some(parent), Some(leaf)) = (path.parent(), path.file_name()) else {
                return Ok(None);
            };
            match parent.canonicalize() {
                Ok(parent) => Ok(Some(parent.join(leaf))),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) if error.kind() == std::io::ErrorKind::NotADirectory => {
                    Err(SymlinkError::TargetNotDirectory)
                }
                Err(error) => Err(SymlinkError::Io(error.kind().to_string())),
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotADirectory => {
            Err(SymlinkError::TargetNotDirectory)
        }
        Err(error) => Err(SymlinkError::Io(error.kind().to_string())),
    }
}

pub fn deploy(link: &Path, target: &Path) -> Result<(), SymlinkError> {
    let existing = match inspect(link) {
        LinkState::LinkTo(value) => {
            let found = prospective(&destination(link, &value))?;
            let expected = prospective(target)?;
            if found.is_none() || found != expected {
                return Err(SymlinkError::Occupied(format!(
                    "a link to {}",
                    value.display()
                )));
            }
            true
        }
        LinkState::Other(what) => return Err(SymlinkError::Occupied(what)),
        LinkState::Unreadable(detail) => return Err(SymlinkError::Io(detail)),
        LinkState::Absent => false,
    };
    let target = ensure_target(target)?;
    if existing {
        return verify(link, &target);
    }
    std::os::unix::fs::symlink(target, link)
        .map_err(|error| SymlinkError::Io(error.kind().to_string()))
}

pub fn verify(link: &Path, target: &Path) -> Result<(), SymlinkError> {
    match inspect(link) {
        LinkState::LinkTo(value) => {
            let found = prospective(&destination(link, &value))?;
            let expected = prospective(target)?;
            if found.is_none() || found != expected {
                return Err(SymlinkError::WrongTarget(value));
            }
            let expected = resolved_directory(target)?;
            let found = resolved_directory(&destination(link, &value))?;
            if found == expected {
                Ok(())
            } else {
                Err(SymlinkError::WrongTarget(value))
            }
        }
        LinkState::Absent => Err(SymlinkError::Missing),
        LinkState::Other(what) => Err(SymlinkError::Occupied(what)),
        LinkState::Unreadable(detail) => Err(SymlinkError::Io(detail)),
    }
}

#[cfg(test)]
mod tests;
