use super::ArchiveReader;
use crate::{Access, ContainedError, RootDir};
use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Placement {
    Cloned,
    Renamed,
    Exists,
}

pub(super) fn place(
    root: &RootDir,
    file: &File,
    staged: &Path,
    target: &Path,
) -> Result<Placement, ContainedError> {
    place_with(root, file, staged, target, RootDir::clone_file)
}

pub(super) fn place_with<C>(
    root: &RootDir,
    file: &File,
    staged: &Path,
    target: &Path,
    clone_at: C,
) -> Result<Placement, ContainedError>
where
    C: FnOnce(&RootDir, &File, &Path) -> std::io::Result<()>,
{
    match clone_at(root, file, target) {
        Ok(()) => Ok(Placement::Cloned),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(Placement::Exists),
        Err(error) if error.raw_os_error() == Some(libc::ENOTSUP) => rename(root, staged, target),
        Err(error) => Err(ContainedError::Io {
            path: target.into(),
            kind: error.kind(),
        }),
    }
}

fn rename(root: &RootDir, staged: &Path, target: &Path) -> Result<Placement, ContainedError> {
    if root.rename_exclusive(staged, target)? {
        Ok(Placement::Renamed)
    } else {
        Ok(Placement::Exists)
    }
}

pub(super) fn remove_staging(
    root: &RootDir,
    handle: &ArchiveReader,
    staged: &Path,
) -> Result<(), ContainedError> {
    let Some(named) = root
        .open_file(staged, Access::Read)
        .ok()
        .and_then(|file| file.metadata().ok())
    else {
        return Ok(());
    };
    if (named.dev(), named.ino()) != (handle.initial.dev(), handle.initial.ino()) {
        return Ok(());
    }
    // A file swapped in at the staging name between this check and the unlink is removed too.
    root.unlink_file(staged)
}
