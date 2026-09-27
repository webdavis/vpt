use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Escape(PathBuf),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreEntry {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: FileTime,
}

pub trait Stores {
    fn entries(&self, root: &Path) -> Result<Vec<StoreEntry>, StoreError>;
    fn validate_path(&self, path: &Path) -> Result<(), StoreError>;
    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError>;
    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest;
    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), StoreError>;
    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError>;
}
