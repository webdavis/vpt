use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Escape(PathBuf),
    Changed(PathBuf),
    EditKept(KeptEdit),
    UnsafeVolume(PathBuf),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptEdit {
    pub note: PathBuf,
    pub kept: PathBuf,
    pub versions: KeptVersions,
    pub cause: Option<Box<StoreError>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptVersions {
    PublishedAtNote,
    NewerSaveAtNote,
    OlderSaveAtNote,
    EditedNote,
    NoteMissing,
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
    /// Files vpt set aside beside the note, each holding a version the note no longer holds.
    fn kept_beside(&self, note: &Path) -> Result<Vec<PathBuf>, StoreError>;
    /// Moves a kept file back to the note's name, or removes it when that name is taken again.
    fn return_kept(&self, kept: &Path, note: &Path) -> Result<(), StoreError>;
    fn remove_kept(&self, kept: &Path) -> Result<(), StoreError>;
}
