use super::{TermsError, digest, snapshot};
use crate::{Access, RootDir};
use std::{io::Write, path::Path};
use vpt_domain::digest::Sha256Digest;
mod temporary;
use temporary::Temporary;

pub(super) fn publish(
    root: &RootDir,
    target: &Path,
    previous: Option<Sha256Digest>,
    bytes: &[u8],
) -> Result<(), TermsError> {
    publish_with(root, target, previous, bytes, |_| Ok(()))
}

#[derive(Clone, Copy)]
enum Checkpoint {
    Staged,
    Renamed,
}

fn publish_with(
    root: &RootDir,
    target: &Path,
    previous: Option<Sha256Digest>,
    bytes: &[u8],
    mut checkpoint: impl FnMut(Checkpoint) -> Result<(), TermsError>,
) -> Result<(), TermsError> {
    let current = snapshot::read(root, target)?;
    if current.as_ref().is_some_and(|current| !current.single_link) {
        return Err(TermsError::UnsafePath);
    }
    let current = current.as_ref().map(|current| current.bytes.as_slice());
    if current == Some(bytes) {
        return finish(root, target, bytes);
    }
    if current.map(digest) != previous {
        return Err(TermsError::Changed);
    }
    let mut temporary = Temporary::create(root)?;
    temporary
        .file
        .write_all(bytes)
        .and_then(|()| temporary.file.sync_all())
        .map_err(|_| TermsError::Unavailable)?;
    checkpoint(Checkpoint::Staged)?;
    let current = snapshot::read(root, target)?;
    if current.as_ref().is_some_and(|current| !current.single_link)
        || current.as_ref().map(|current| digest(&current.bytes)) != previous
    {
        return Err(TermsError::Changed);
    }
    temporary.validate(bytes)?;
    if previous.is_none() {
        if !root
            .rename_exclusive(&temporary.path, target)
            .map_err(|_| TermsError::Unavailable)?
        {
            return Err(TermsError::Changed);
        }
    } else {
        root.rename_over(&temporary.path, target)
            .map_err(|_| TermsError::Unavailable)?;
    }
    temporary.published = true;
    checkpoint(Checkpoint::Renamed)?;
    finish(root, target, bytes)
}

#[cfg(test)]
mod tests;

fn finish(root: &RootDir, target: &Path, bytes: &[u8]) -> Result<(), TermsError> {
    root.open_file(target, Access::Read)
        .map_err(|_| TermsError::Changed)?
        .sync_all()
        .map_err(|_| TermsError::Unavailable)?;
    root.sync().map_err(|_| TermsError::Unavailable)?;
    let current = snapshot::read(root, target)?;
    if current
        .as_ref()
        .is_none_or(|current| !current.single_link || current.bytes != bytes)
    {
        return Err(TermsError::Changed);
    }
    Ok(())
}
