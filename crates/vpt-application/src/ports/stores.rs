use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Escape(PathBuf),
    Io(String),
}

pub trait Stores {
    fn validate_path(&self, path: &Path) -> Result<(), StoreError>;
    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError>;
    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest;
    fn publish(&self, target: &Path, bytes: &[u8]) -> Result<(), StoreError>;
    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError>;
}
