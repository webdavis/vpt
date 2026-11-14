use super::placement::{self, Placement};
use super::{ArchiveReader, archive_error};
use crate::{ContainedError, RootDir};
use std::fs::File;
use std::path::Path;
use vpt_application::ports::{ArchiveError, Published, Staged};

pub(super) fn exclusive(
    root: &RootDir,
    staged: &Staged,
    handle: &ArchiveReader,
    target_name: &str,
) -> Result<Published, ArchiveError> {
    exclusive_with(
        root,
        staged,
        handle,
        target_name,
        sync_open,
        sync_directory,
        placement::place,
    )
}

pub(super) fn exclusive_with<F, D, P>(
    root: &RootDir,
    staged: &Staged,
    handle: &ArchiveReader,
    target_name: &str,
    mut sync_file: F,
    sync_directory: D,
    place: P,
) -> Result<Published, ArchiveError>
where
    F: FnMut(&File) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
    P: FnOnce(&RootDir, &File, &Path, &Path) -> Result<Placement, ContainedError>,
{
    let target = root
        .leaf(Path::new(target_name))
        .map_err(|error| archive_error(&root.path().join(target_name), error))?;
    root.revalidate()
        .map_err(|error| archive_error(&target, error))?;
    if staged.path != handle.path || staged.size != handle.initial.len() {
        return Err(ArchiveError::Changed(staged.path.clone()));
    }
    handle.verify_publication(root, &staged.path, &staged.digest)?;
    sync_file(&handle.file)?;
    handle.verify_publication(root, &staged.path, &staged.digest)?;
    let placement = place(root, &handle.file, &staged.path, &target)
        .map_err(|error| archive_error(&target, error))?;
    if placement == Placement::Exists {
        root.regular(&target)
            .map_err(|error| archive_error(&target, error))?;
        root.revalidate()
            .map_err(|error| archive_error(&target, error))?;
        return Ok(Published::Exists(target));
    }
    confirm(
        root,
        staged,
        handle,
        &target,
        placement,
        sync_file,
        sync_directory,
    )
    .map_err(|error| match error {
        ArchiveError::Changed(_) | ArchiveError::Escape(_) => error,
        _ => ArchiveError::PlacedUnsynced(target.clone()),
    })?;
    if placement == Placement::Cloned
        && placement::remove_staging(root, handle, &staged.path).is_err()
    {
        return Ok(Published::PlacedLeavingStage(target));
    }
    Ok(Published::Placed(target))
}

fn confirm<F, D>(
    root: &RootDir,
    staged: &Staged,
    handle: &ArchiveReader,
    target: &Path,
    placement: Placement,
    mut sync_file: F,
    sync_directory: D,
) -> Result<(), ArchiveError>
where
    F: FnMut(&File) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
{
    let unsynced = |_| ArchiveError::PlacedUnsynced(target.into());
    let archive = match placement {
        Placement::Cloned => Some(ArchiveReader::open(root, target)?),
        Placement::Renamed | Placement::Exists => None,
    };
    if let Some(archive) = &archive {
        sync_file(&archive.file).map_err(unsynced)?;
    }
    sync_directory(root).map_err(unsynced)?;
    root.revalidate()
        .map_err(|error| archive_error(target, error))?;
    archive
        .as_ref()
        .unwrap_or(handle)
        .verify_publication(root, target, &staged.digest)
}

pub(super) fn sync_file_and_directory(
    root: &RootDir,
    path: &Path,
    expected: &vpt_domain::digest::Sha256Digest,
) -> Result<(), ArchiveError> {
    sync_file_and_directory_with(root, path, expected, sync_open, sync_directory)
}

pub(super) fn sync_file_and_directory_with<F, D>(
    root: &RootDir,
    path: &Path,
    expected: &vpt_domain::digest::Sha256Digest,
    sync_file: F,
    sync_directory: D,
) -> Result<(), ArchiveError>
where
    F: FnOnce(&std::fs::File) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
{
    super::verified::sync_with(root, path, expected, sync_file, sync_directory)
}

fn sync_open(file: &std::fs::File) -> Result<(), ArchiveError> {
    file.sync_all()
        .map_err(|error| ArchiveError::Sync(error.kind().to_string()))
}

fn sync_directory(root: &RootDir) -> Result<(), ArchiveError> {
    root.sync().map_err(|error| match error {
        ContainedError::Io { kind, .. } => ArchiveError::Sync(kind.to_string()),
        other => archive_error(root.path(), other),
    })
}

#[cfg(test)]
mod tests;
