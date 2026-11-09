use crate::{Access, RootDir};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::Path;
use vpt_domain::{
    container::{self, ReadFailure},
    digest::Sha256Digest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    UnsafePath,
    Unavailable,
    TooLarge,
    Changed,
    InvalidContainer,
}

pub fn inspect_archived_audio(
    root: &Path,
    path: &Path,
    expected: Sha256Digest,
    maximum: u64,
) -> Result<f64, AudioError> {
    let root = RootDir::open(root).map_err(|_| AudioError::UnsafePath)?;
    root.leaf(path).map_err(|_| AudioError::UnsafePath)?;
    let file = root
        .open_file(path, Access::Read)
        .map_err(|_| AudioError::Unavailable)?;
    let before = file.metadata().map_err(|_| AudioError::Unavailable)?;
    if before.len() > maximum {
        return Err(AudioError::TooLarge);
    }
    let container = container::inspect(before.len(), |offset, buffer| {
        file.read_exact_at(buffer, offset).map_err(|_| ReadFailure)
    })
    .map_err(|_| AudioError::InvalidContainer)?;
    let mut reader = file.take(before.len().saturating_add(1));
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut length = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| AudioError::Unavailable)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        length += read as u64;
    }
    let after = reader
        .get_ref()
        .metadata()
        .map_err(|_| AudioError::Unavailable)?;
    let named = root
        .open_file(path, Access::Read)
        .map_err(|_| AudioError::Changed)?
        .metadata()
        .map_err(|_| AudioError::Changed)?;
    let identity = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if length != before.len()
        || identity(&before) != identity(&after)
        || identity(&after) != identity(&named)
        || Sha256Digest(digest.finalize().into()) != expected
    {
        return Err(AudioError::Changed);
    }
    root.revalidate().map_err(|_| AudioError::UnsafePath)?;
    Ok(container.precise_duration_secs())
}
