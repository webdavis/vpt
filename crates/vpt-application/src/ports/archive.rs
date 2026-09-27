use std::path::{Path, PathBuf};
use vpt_domain::container::ReadFailure;
use vpt_domain::digest::Sha256Digest;

use super::{CloneError, CloneKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub path: PathBuf,
    pub digest: Sha256Digest,
    pub size: u64,
    pub copy_on_write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveError {
    NoSpace,
    Sync(String),
    PlacedUnsynced(PathBuf),
    Escape(PathBuf),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageFailure {
    pub cause: ArchiveError,
    pub owned_staging: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    Placed(PathBuf),
    Exists(PathBuf),
}

pub trait Archive {
    type Handle;

    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>;
    fn open(&self, path: &Path) -> Result<Self::Handle, ArchiveError>;
    fn size(&self, handle: &Self::Handle) -> Result<u64, ArchiveError>;
    fn read_at(
        &self,
        handle: &Self::Handle,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<(), ReadFailure>;
    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError>;
    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError>;
    fn target(&self, name: &str) -> PathBuf;
    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError>;
    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError>;
    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError>;
    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError>;
}
