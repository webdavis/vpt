use std::path::{Path, PathBuf};
use vpt_domain::container::ReadFailure;
use vpt_domain::time::FileTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
    pub mtime: FileTime,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMetadata {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub mtime: FileTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecorderError {
    Unreadable(String),
    NotRegular(PathBuf),
    NotFound(PathBuf),
    Escape(PathBuf),
    Io(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TitleLookup {
    Titled(String),
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneKind {
    CopyOnWrite,
    ByteCopy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneError {
    NoSpace,
    Exists,
    Escape(PathBuf),
    Io(String),
}

pub trait RecorderStore {
    type Handle;

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError>;
    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError>;
    fn open(&self, path: &Path) -> Result<Self::Handle, RecorderError>;
    fn metadata(&self, handle: &Self::Handle) -> Result<SourceMetadata, RecorderError>;
    fn digest(
        &self,
        handle: &Self::Handle,
        size: u64,
    ) -> Result<vpt_domain::digest::Sha256Digest, RecorderError>;
    fn read_at(
        &self,
        handle: &Self::Handle,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<(), ReadFailure>;
    fn clone_into(
        &self,
        handle: &Self::Handle,
        directory: &Path,
        directory_identity: (u64, u64),
        name: &str,
    ) -> Result<CloneKind, CloneError>;
    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)>;
    fn title(&self, file_name: &str) -> TitleLookup;
    fn refresh_titles(&self) -> Result<(), RecorderError>;
}
