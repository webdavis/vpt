use super::ReleaseFileError;
use crate::{Access, ContainedError, FilesystemStores, contained::effective_user, digest_open};
use std::{fs::Metadata, os::unix::fs::MetadataExt, path::Path};
use vpt_application::ports::StoreError;
use vpt_domain::digest::Sha256Digest;

pub(super) fn check(
    stores: &FilesystemStores,
    path: &Path,
    length: usize,
    digest: Sha256Digest,
    private: bool,
    durable: bool,
) -> Result<bool, ReleaseFileError> {
    let (root, leaf) = stores.owner(path)?;
    let mut file = match root.open_file(&leaf, Access::Read) {
        Ok(file) => file,
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => return Ok(false),
        Err(error) => return Err(super::super::contained(path, error)),
    };
    let io = |e: std::io::Error| ReleaseFileError::Store(StoreError::Io(e.kind().to_string()));
    let metadata = file.metadata().map_err(io)?;
    validate(&metadata, path, length, private)?;
    if digest_open(&mut file).map_err(io)? != digest {
        return Err(StoreError::Changed(path.into()).into());
    }
    if durable {
        file.sync_all().map_err(io)?;
        root.sync().map_err(|e| super::super::contained(path, e))?;
    }
    let mut current = root
        .open_file(&leaf, Access::Read)
        .map_err(|e| super::super::contained(path, e))?;
    let after = current.metadata().map_err(io)?;
    validate(&after, path, length, private)?;
    if metadata.dev() != after.dev()
        || metadata.ino() != after.ino()
        || digest_open(&mut current).map_err(io)? != digest
    {
        return Err(StoreError::Changed(path.into()).into());
    }
    root.revalidate()
        .map_err(|e| super::super::contained(path, e))?;
    Ok(true)
}
fn validate(
    meta: &Metadata,
    path: &Path,
    length: usize,
    private: bool,
) -> Result<(), ReleaseFileError> {
    if private
        && (meta.uid() != effective_user() || meta.mode() & 0o7777 != 0o600 || meta.nlink() != 1)
    {
        return Err(ReleaseFileError::PrivateReport);
    }
    if meta.len() != length as u64 {
        return Err(StoreError::Changed(path.into()).into());
    }
    Ok(())
}
