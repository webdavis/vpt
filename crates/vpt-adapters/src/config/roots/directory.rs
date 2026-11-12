use super::{RootError, io};
use crate::{ContainedError, RootDir};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub fn planned_directory(path: &Path, key: &str) -> Result<PathBuf, RootError> {
    if !path.is_absolute() {
        return Err(RootError::NotAbsolute { key: key.into() });
    }
    let mut cursor = path;
    let mut missing = Vec::new();
    loop {
        match existing_directory(cursor, key)? {
            Some(mut parent) => {
                for leaf in missing.iter().rev() {
                    parent.push(leaf);
                }
                return Ok(parent);
            }
            None => {
                let leaf = cursor
                    .file_name()
                    .ok_or_else(|| RootError::UnresolvedParent { key: key.into() })?;
                missing.push(leaf.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| RootError::UnresolvedParent { key: key.into() })?;
            }
        }
    }
}

pub(super) fn existing_directory(path: &Path, key: &str) -> Result<Option<PathBuf>, RootError> {
    match path.canonicalize() {
        Ok(canonical) => {
            if !canonical.is_dir() {
                return Err(RootError::NotADirectory { key: key.into() });
            }
            Ok(Some(canonical))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => match path.symlink_metadata() {
            Ok(_) => Err(RootError::PathEscape { key: key.into() }),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io(key, error)),
        },
        Err(error) if error.kind() == ErrorKind::NotADirectory => {
            Err(RootError::NotADirectory { key: key.into() })
        }
        Err(error) => Err(io(key, error)),
    }
}

pub(super) fn private_dir(path: &Path, key: &str) -> Result<(), RootError> {
    let invalid = || RootError::NotAbsolute { key: key.into() };
    let mut names = Vec::new();
    let mut existing = path;
    loop {
        let name = existing.file_name().and_then(|name| name.to_str());
        names.push(name.ok_or_else(invalid)?);
        existing = existing.parent().ok_or_else(invalid)?;
        match existing.symlink_metadata() {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            _ => break,
        }
    }
    let map = |error| match error {
        ContainedError::Io { kind, .. } => RootError::Io {
            key: key.into(),
            detail: kind.to_string(),
        },
        _ => RootError::PathEscape { key: key.into() },
    };
    let mut directory = RootDir::open(existing).map_err(map)?;
    for name in names.into_iter().rev() {
        directory = directory.subdirectory(name).map_err(map)?;
    }
    Ok(())
}
