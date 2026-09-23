use super::{RootError, io};
use std::fs::DirBuilder;
use std::io::ErrorKind;
use std::os::unix::fs::DirBuilderExt;
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
                    .ok_or_else(|| RootError::ParentMissing { key: key.into() })?;
                missing.push(leaf.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| RootError::ParentMissing { key: key.into() })?;
            }
        }
    }
}

pub(super) fn prospective(
    path: &Path,
    key: &str,
    below_home: Option<PathBuf>,
) -> Result<PathBuf, RootError> {
    if !path.is_absolute() {
        return Err(RootError::NotAbsolute { key: key.into() });
    }
    if let Some(canonical) = existing_directory(path, key)? {
        return Ok(canonical);
    }
    let parent = path
        .parent()
        .ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    let leaf = path
        .file_name()
        .ok_or_else(|| RootError::NotAbsolute { key: key.into() })?;
    match existing_directory(parent, key)? {
        Some(parent) => Ok(parent.join(leaf)),
        None => below_home.ok_or_else(|| RootError::ParentMissing { key: key.into() }),
    }
}

fn existing_directory(path: &Path, key: &str) -> Result<Option<PathBuf>, RootError> {
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
    match DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
        Err(error) => Err(io(key, error)),
    }
}
