use super::ReleaseFileError;
use crate::{RootDir, Roots, config::planned_directory};
use std::path::{Component, Path, PathBuf};
use vpt_domain::layout::StoreKey;

pub(super) fn resolve(roots: &Roots, path: &Path) -> Result<PathBuf, ReleaseFileError> {
    let destination = canonical(path)?;
    for key in StoreKey::all().into_iter().filter(|key| key.is_private()) {
        if destination.starts_with(canonical(roots.stores.get(key))?) {
            return Err(ReleaseFileError::Protected(key.key_name()));
        }
    }
    for (path, name) in [
        (&roots.state_dir, "state"),
        (&roots.config_dir, "config"),
        (&roots.container, "voice_memos"),
    ] {
        if destination.starts_with(canonical(path)?) {
            return Err(ReleaseFileError::Protected(name));
        }
    }
    Ok(destination)
}

pub(super) fn canonical(path: &Path) -> Result<PathBuf, ReleaseFileError> {
    if path.components().count() > 256 {
        return Err(ReleaseFileError::Capacity);
    }
    let path =
        planned_directory(path, "share.destination").map_err(|_| ReleaseFileError::Invalid)?;
    if path
        .components()
        .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return Err(ReleaseFileError::Invalid);
    }
    Ok(path)
}

pub(super) fn create(path: &Path) -> Result<RootDir, ReleaseFileError> {
    let mut cursor = path;
    let mut missing = Vec::new();
    let mut root = loop {
        match RootDir::open(cursor) {
            Ok(root) => break root,
            Err(crate::ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => {
                missing.push(cursor.file_name().ok_or(ReleaseFileError::Invalid)?);
                cursor = cursor.parent().ok_or(ReleaseFileError::Invalid)?;
            }
            Err(error) => return Err(super::contained(cursor, error)),
        }
    };
    for name in missing.into_iter().rev() {
        root.revalidate().map_err(|e| super::contained(path, e))?;
        let child = root
            .subdirectory(name.to_str().ok_or(ReleaseFileError::Invalid)?)
            .map_err(|e| super::contained(path, e))?;
        root.sync().map_err(|e| super::contained(path, e))?;
        root = child;
    }
    root.revalidate().map_err(|e| super::contained(path, e))?;
    Ok(root)
}
