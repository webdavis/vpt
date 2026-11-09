use super::*;
pub(super) mod read;
use vpt_application::ports::{NoteFiles, StoredNote};

impl NoteFiles for FilesystemStores {
    fn note_entries(
        &self,
        root: &Path,
    ) -> Result<vpt_application::ports::RetentionEntries, StoreError> {
        self.scan_bounded(root, 65_536)
    }

    fn read_note(&self, path: &Path) -> Result<Option<StoredNote>, StoreError> {
        self.read_note_with(path, || {})
    }
}

impl FilesystemStores {
    fn read_note_with(
        &self,
        path: &Path,
        after_read: impl FnOnce(),
    ) -> Result<Option<StoredNote>, StoreError> {
        self.read_note_with_limit(path, vpt_domain::note::MAX_RENDERED, None, after_read)
    }

    pub fn read_note_version(
        &self,
        path: &Path,
        expected: Sha256Digest,
    ) -> Result<Option<StoredNote>, StoreError> {
        self.read_note_with_limit(path, 16 * 1024 * 1024, Some(expected), || {})
    }

    fn read_note_with_limit(
        &self,
        path: &Path,
        maximum: usize,
        expected: Option<Sha256Digest>,
        after_read: impl FnOnce(),
    ) -> Result<Option<StoredNote>, StoreError> {
        let bytes = self
            .read_note_bytes(path, maximum, expected, after_read)
            .map_err(|error| match error {
                read::ReadError::Store(error) => error,
                read::ReadError::TooLarge if expected.is_some() => StoreError::Changed(path.into()),
                read::ReadError::TooLarge => StoreError::Io("note exceeds size limit".into()),
            })?;
        bytes
            .map(|row| {
                let text = String::from_utf8(row.bytes)
                    .map_err(|_| StoreError::Io("note is not UTF-8".into()))?;
                Ok(StoredNote {
                    text,
                    digest: row.digest,
                })
            })
            .transpose()
    }
}
