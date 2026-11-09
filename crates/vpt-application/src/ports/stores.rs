use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Escape(PathBuf),
    Changed(PathBuf),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreEntry {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: FileTime,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetentionEntries {
    pub files: Vec<StoreEntry>,
    pub redirected: Vec<PathBuf>,
}

pub trait Stores {
    fn retention_entries(&self, root: &Path) -> Result<RetentionEntries, StoreError> {
        self.entries(root).map(|files| RetentionEntries {
            files,
            redirected: vec![],
        })
    }

    fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, StoreError>;
    fn validate_path(&self, path: &Path) -> Result<(), StoreError>;
    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError>;
    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest;
    /// Recheck expected bytes before publication. An absent expectation requires exclusive creation.
    fn publish(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
        bytes: &[u8],
    ) -> Result<(), StoreError>;
    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError>;
}
