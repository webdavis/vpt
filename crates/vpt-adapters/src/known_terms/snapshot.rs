use super::{MAX_BYTES, TermsError, missing};
use crate::{Access, ContainedError, RootDir};
use std::{fs::Metadata, io::Read, os::unix::fs::MetadataExt, path::Path};

pub(super) struct Snapshot {
    pub(super) bytes: Vec<u8>,
    pub(super) single_link: bool,
}

pub(super) fn read(root: &RootDir, path: &Path) -> Result<Option<Snapshot>, TermsError> {
    root.revalidate().map_err(|_| TermsError::Changed)?;
    let file = match root.open_file(path, Access::Read) {
        Ok(file) => file,
        Err(error) if missing(&error) => {
            root.revalidate().map_err(|_| TermsError::Changed)?;
            return Ok(None);
        }
        Err(ContainedError::NotRegular(_)) => return Err(TermsError::UnsafePath),
        Err(_) => return Err(TermsError::Unavailable),
    };
    let before = file.metadata().map_err(|_| TermsError::Unavailable)?;
    if before.len() > MAX_BYTES as u64 {
        return Err(TermsError::TooLarge);
    }
    let mut reader = file.take(before.len() + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| TermsError::Unavailable)?;
    let after = reader
        .get_ref()
        .metadata()
        .map_err(|_| TermsError::Unavailable)?;
    let named = root
        .open_file(path, Access::Read)
        .map_err(|_| TermsError::Changed)?
        .metadata()
        .map_err(|_| TermsError::Changed)?;
    if bytes.len() as u64 != before.len()
        || identity(&before) != identity(&after)
        || identity(&after) != identity(&named)
    {
        return Err(TermsError::Changed);
    }
    root.revalidate().map_err(|_| TermsError::Changed)?;
    Ok(Some(Snapshot {
        bytes,
        single_link: after.nlink() == 1,
    }))
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
