use crate::contained::{Access, ContainedError, Kind, RootDir};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use vpt_application::ports::{RetentionEntries, StoreEntry, StoreError, Stores};
use vpt_domain::digest::Sha256Digest;
mod inventory;
mod notes;
mod publication;
mod retention;
mod roots;

mod digest;
pub(crate) use digest::digest_reader;
pub use digest::{BUFFER, digest_open};

pub struct FilesystemStores {
    roots: Vec<RootDir>,
    configured: Vec<PathBuf>,
    private: Vec<PathBuf>,
    sequence: AtomicU64,
}

fn store_error(path: &Path, error: ContainedError) -> StoreError {
    match error {
        ContainedError::Io { kind, .. } => StoreError::Io(format!("{kind} at {}", path.display())),
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => StoreError::Escape(path.to_path_buf()),
    }
}

fn io(path: &Path, error: &std::io::Error) -> StoreError {
    StoreError::Io(format!("{} at {}", error.kind(), path.display()))
}

impl Stores for FilesystemStores {
    fn entries(&self, path: &Path) -> Result<Vec<StoreEntry>, StoreError> {
        let scanned = self.scan(path)?;
        if let Some(path) = scanned.redirected.first() {
            return Err(StoreError::Escape(path.clone()));
        }
        Ok(scanned.files)
    }

    fn retention_entries(&self, path: &Path) -> Result<RetentionEntries, StoreError> {
        self.scan_bounded(path, 65_536)
    }

    fn validate_path(&self, path: &Path) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(path)?;
        match root.stat(&leaf) {
            Ok(stat) if stat.kind == Kind::File => Ok(()),
            Ok(_) => Err(StoreError::Escape(path.to_path_buf())),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => Ok(()),
            Err(error) => Err(store_error(path, error)),
        }
    }

    fn digest_of(&self, path: &Path) -> Result<Option<Sha256Digest>, StoreError> {
        let (root, leaf) = self.owner(path)?;
        match root.open_file(&leaf, Access::Read) {
            Ok(mut file) => digest_open(&mut file)
                .map(Some)
                .map_err(|error| io(path, &error)),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => Ok(None),
            Err(error) => Err(store_error(path, error)),
        }
    }

    fn digest_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        Sha256Digest(Sha256::digest(bytes).into())
    }

    fn publish(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        self.publish_bytes(target, expected, bytes)
    }

    fn sync_directory_of(&self, path: &Path) -> Result<(), StoreError> {
        let (root, _leaf) = self.owner(path)?;
        root.sync().map_err(|error| store_error(path, error))
    }
}

#[cfg(test)]
mod tests;
