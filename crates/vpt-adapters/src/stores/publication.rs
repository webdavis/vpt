use super::{FilesystemStores, Stores, io, store_error};
use crate::contained::{ContainedError, Placement, RenameSupport, RootDir};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use vpt_application::ports::StoreError;
use vpt_domain::digest::Sha256Digest;
mod displaced;
mod kept;
mod kept_list;
mod place;
mod replace;
mod restore;
mod staged;
pub use kept_list::kept_files;

#[derive(Clone, Copy)]
enum Checkpoint {
    Staged,
    Checked,
    Displaced,
    Hashed,
    Linked,
    SwapBack,
    Renamed,
}

impl FilesystemStores {
    fn create_temporary(&self, root: &RootDir, name: &str) -> Result<(File, PathBuf), StoreError> {
        let mode = if self.private.iter().any(|path| path == root.path()) {
            0o600
        } else {
            0o644
        };
        for _ in 0..64 {
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
            let temporary = PathBuf::from(format!(".{name}.vpt-{}-{sequence}", std::process::id()));
            match root.create_file(&temporary, mode) {
                Ok(file) => return Ok((file, temporary)),
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::AlreadyExists,
                    ..
                }) => continue,
                Err(error) => return Err(store_error(&root.path().join(temporary), error)),
            }
        }
        Err(StoreError::Io(
            "publication temporary names exhausted".into(),
        ))
    }
    pub(super) fn publish_bytes(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        let support = self.owner(target)?.0.rename_support();
        self.publish_with(target, expected, bytes, support, |_| Ok(()))
    }

    fn publish_with(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
        bytes: &[u8],
        support: RenameSupport,
        mut checkpoint: impl FnMut(Checkpoint) -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(target)?;
        if !support.places_exclusively() {
            return Err(StoreError::UnsafeVolume(target.to_path_buf()));
        }
        self.check_previous(target, expected)?;
        let name = leaf
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| StoreError::Escape(target.to_path_buf()))?;
        let (mut file, temporary) = self.create_temporary(root, name)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| io(target, &error))?;
        checkpoint(Checkpoint::Staged)?;
        self.check_previous(target, expected)?;
        staged::validate(root, &temporary, &file, bytes)?;
        checkpoint(Checkpoint::Checked)?;
        match expected {
            None => {
                let placement =
                    place::place_exclusive(root, support, &temporary, &leaf, &mut checkpoint)?;
                if placement == Placement::Occupied {
                    return Err(StoreError::Changed(target.to_path_buf()));
                }
            }
            Some(expected) => replace::Replacement {
                stores: self,
                root,
                support,
                target,
                name,
                note: &leaf,
                staged: &temporary,
                file: &file,
                bytes,
                expected,
            }
            .run(&mut checkpoint)?,
        }
        checkpoint(Checkpoint::Renamed)?;
        root.sync().map_err(|error| store_error(target, error))?;
        if self.digest_of(target)? != Some(self.digest_bytes(bytes)) {
            return Err(StoreError::Changed(target.to_path_buf()));
        }
        Ok(())
    }

    fn check_previous(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
    ) -> Result<(), StoreError> {
        if self.digest_of(target)? != expected {
            return Err(StoreError::Changed(target.to_path_buf()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
