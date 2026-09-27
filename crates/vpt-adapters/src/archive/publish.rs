use super::archive_error;
use crate::{Access, ContainedError, RootDir};
use std::path::Path;
use vpt_application::ports::{ArchiveError, Published};

pub(super) fn exclusive(
    root: &RootDir,
    staged: &Path,
    target_name: &str,
) -> Result<Published, ArchiveError> {
    exclusive_with(root, staged, target_name, sync_file, sync_directory)
}

pub(super) fn exclusive_with<F, D>(
    root: &RootDir,
    staged: &Path,
    target_name: &str,
    sync_file: F,
    sync_directory: D,
) -> Result<Published, ArchiveError>
where
    F: FnOnce(&RootDir, &Path) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
{
    let target = root
        .leaf(Path::new(target_name))
        .map_err(|error| archive_error(&root.path().join(target_name), error))?;
    root.revalidate()
        .map_err(|error| archive_error(&target, error))?;
    sync_file(root, staged)?;
    let placed = root
        .rename_exclusive(staged, &target)
        .map_err(|error| archive_error(&target, error))?;
    if !placed {
        root.regular(&target)
            .map_err(|error| archive_error(&target, error))?;
        root.revalidate()
            .map_err(|error| archive_error(&target, error))?;
        return Ok(Published::Exists(target));
    }
    sync_directory(root).map_err(|_| ArchiveError::PlacedUnsynced(target.clone()))?;
    root.revalidate()
        .map_err(|error| archive_error(&target, error))?;
    Ok(Published::Placed(target))
}

pub(super) fn sync_file_and_directory(root: &RootDir, path: &Path) -> Result<(), ArchiveError> {
    sync_file_and_directory_with(root, path, sync_file, sync_directory)
}

pub(super) fn sync_file_and_directory_with<F, D>(
    root: &RootDir,
    path: &Path,
    sync_file: F,
    sync_directory: D,
) -> Result<(), ArchiveError>
where
    F: FnOnce(&RootDir, &Path) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
{
    root.revalidate()
        .map_err(|error| archive_error(path, error))?;
    sync_file(root, path)?;
    sync_directory(root)?;
    root.revalidate()
        .map_err(|error| archive_error(path, error))
}

fn sync_file(root: &RootDir, path: &Path) -> Result<(), ArchiveError> {
    let file = root
        .open_file(path, Access::Read)
        .map_err(|error| match error {
            ContainedError::Io { kind, .. } => ArchiveError::Sync(kind.to_string()),
            other => archive_error(path, other),
        })?;
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
