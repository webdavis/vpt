use super::*;
use std::{io::Read, os::unix::fs::MetadataExt};

pub(in crate::stores) struct StoredBytes {
    pub bytes: Vec<u8>,
    pub digest: Sha256Digest,
}

pub(in crate::stores) enum ReadError {
    Store(StoreError),
    TooLarge,
}
impl From<StoreError> for ReadError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl FilesystemStores {
    pub(in crate::stores) fn read_note_bytes(
        &self,
        path: &Path,
        maximum: usize,
        expected: Option<Sha256Digest>,
        after_read: impl FnOnce(),
    ) -> Result<Option<StoredBytes>, ReadError> {
        let (root, leaf) = self.owner(path)?;
        let file = match root.open_file(&leaf, Access::Read) {
            Ok(file) => file,
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => {
                root.revalidate()
                    .map_err(|error| store_error(path, error))?;
                return Ok(None);
            }
            Err(error) => return Err(store_error(path, error).into()),
        };
        let before = file.metadata().map_err(|error| io(path, &error))?;
        if before.len() > maximum as u64 {
            return Err(ReadError::TooLarge);
        }
        let mut reader = file.take(before.len() + 1);
        let mut bytes = Vec::with_capacity(before.len() as usize + 1);
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| io(path, &error))?;
        after_read();
        let after = reader
            .get_ref()
            .metadata()
            .map_err(|error| io(path, &error))?;
        let named = root
            .open_file(&leaf, Access::Read)
            .map_err(|_| changed())?
            .metadata()
            .map_err(|error| io(path, &error))?;
        if bytes.len() as u64 != before.len() || !same(&before, &after) || !same(&after, &named) {
            return Err((if expected.is_some() {
                StoreError::Changed(path.into())
            } else {
                changed()
            })
            .into());
        }
        root.revalidate()
            .map_err(|error| store_error(path, error))?;
        let digest = self.digest_bytes(&bytes);
        if expected.is_some_and(|expected| expected != digest) {
            return Err(StoreError::Changed(path.into()).into());
        }
        Ok(Some(StoredBytes { bytes, digest }))
    }
}

fn changed() -> StoreError {
    StoreError::Io("note changed during read".into())
}

fn same(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
        && left.nlink() == right.nlink()
}
