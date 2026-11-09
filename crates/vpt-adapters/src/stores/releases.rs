use super::FilesystemStores;
use crate::{RootDir, Roots};
use vpt_application::ports::Stores;
mod destination;
mod naming;
mod publication;
const MAX_FILES: usize = 65_536;
const MAX_TEXT: usize = 16 * 1024 * 1024;
use std::path::{Path, PathBuf};
use vpt_application::ports::StoreError;
use vpt_domain::{
    identity::RecordingId,
    note::NoteStage,
    time::{UtcInstant, UtcOffset},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ReleaseFileError {
    Invalid,
    Protected(&'static str),
    Duplicate(PathBuf),
    PrivateReport,
    Capacity,
    Store(StoreError),
}
#[derive(Debug, PartialEq, Eq)]
pub struct ReleaseTarget {
    pub path: PathBuf,
    pub collision: bool,
}
pub struct ReleaseFiles {
    stores: FilesystemStores,
    destination: PathBuf,
    drafts: PathBuf,
}
impl ReleaseFiles {
    pub fn planned_destination(
        roots: &Roots,
        destination: &Path,
    ) -> Result<PathBuf, ReleaseFileError> {
        destination::resolve(roots, destination)
    }

    /// The caller holds VPT's write lock through selection, reservation, and publication.
    pub fn open(roots: &Roots, destination: &Path) -> Result<Self, ReleaseFileError> {
        let destination = Self::planned_destination(roots, destination)?;
        let drafts = destination::canonical(&roots.stores.drafts)?;
        let private = RootDir::open(&drafts).map_err(|e| contained(&drafts, e))?;
        let released = destination::create(&destination)?;
        let stores = FilesystemStores {
            roots: vec![released, private],
            configured: vec![destination.clone(), drafts.clone()],
            private: vec![drafts.clone()],
            sequence: std::sync::atomic::AtomicU64::new(0),
        };
        Ok(Self {
            stores,
            destination,
            drafts,
        })
    }
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    pub fn select(
        &self,
        id: &RecordingId,
        stage: NoteStage,
        bytes: &[u8],
        at: UtcInstant,
        offset: UtcOffset,
    ) -> Result<ReleaseTarget, ReleaseFileError> {
        if bytes.len() > MAX_TEXT {
            return Err(ReleaseFileError::Capacity);
        }
        let digest = self.stores.digest_bytes(bytes);
        let entries = self.stores.scan_bounded(&self.destination, MAX_FILES)?;
        for entry in entries.files {
            if self.stores.digest_of(&entry.path)? == Some(digest) {
                return Err(ReleaseFileError::Duplicate(entry.path));
            }
        }
        naming::select(self, id, stage, at, offset)
    }
}
impl From<StoreError> for ReleaseFileError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
fn contained(path: &Path, error: crate::ContainedError) -> ReleaseFileError {
    super::store_error(path, error).into()
}
#[cfg(test)]
mod tests;
