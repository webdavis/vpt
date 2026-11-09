use super::archive_error;
use crate::{Access, ContainedError, RootDir};
use std::fs::{File, Metadata};
use std::io::{Read, Seek};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use vpt_application::ports::ArchiveError;
use vpt_domain::digest::Sha256Digest;

pub(super) fn sync_with<F, D>(
    root: &RootDir,
    path: &Path,
    expected: &Sha256Digest,
    sync_file: F,
    sync_directory: D,
) -> Result<(), ArchiveError>
where
    F: FnOnce(&File) -> Result<(), ArchiveError>,
    D: FnOnce(&RootDir) -> Result<(), ArchiveError>,
{
    root.revalidate()
        .map_err(|error| archive_error(path, error))?;
    let mut file = root
        .open_file(path, Access::Read)
        .map_err(|error| match error {
            ContainedError::Io { kind, .. } => ArchiveError::Sync(kind.to_string()),
            other => archive_error(path, other),
        })?;
    let before = file.metadata().map_err(io)?;
    verify(root, path, &mut file, &before, expected)?;
    sync_file(&file)?;
    sync_directory(root)?;
    root.revalidate()
        .map_err(|error| archive_error(path, error))?;
    verify(root, path, &mut file, &before, expected)
}

fn verify(
    root: &RootDir,
    path: &Path,
    file: &mut File,
    before: &Metadata,
    expected: &Sha256Digest,
) -> Result<(), ArchiveError> {
    if digest_stable(root, path, file, before)? != *expected {
        return Err(changed());
    }
    Ok(())
}

pub(super) fn digest_stable(
    root: &RootDir,
    path: &Path,
    file: &mut File,
    before: &Metadata,
) -> Result<Sha256Digest, ArchiveError> {
    file.rewind().map_err(io)?;
    let digest = crate::stores::digest_reader(&mut (&mut *file).take(before.len())).map_err(io)?;
    let after = file.metadata().map_err(io)?;
    let named = root
        .open_file(path, Access::Read)
        .map_err(|error| archive_error(path, error))?
        .metadata()
        .map_err(io)?;
    if identity(before) != identity(&after) || identity(before) != identity(&named) {
        return Err(changed());
    }
    root.revalidate()
        .map_err(|error| archive_error(path, error))?;
    Ok(digest)
}

fn changed() -> ArchiveError {
    ArchiveError::Sync("archive changed since verification".into())
}

fn identity(metadata: &Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

fn io(error: std::io::Error) -> ArchiveError {
    ArchiveError::Sync(error.kind().to_string())
}
