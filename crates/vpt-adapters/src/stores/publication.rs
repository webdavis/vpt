use super::{FilesystemStores, Stores, io, store_error};
use crate::contained::{ContainedError, RootDir};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use vpt_application::ports::StoreError;
use vpt_domain::digest::Sha256Digest;
mod staged;

#[derive(Clone, Copy)]
enum Checkpoint {
    Staged,
    Checked,
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
        self.publish_with(target, expected, bytes, |_| Ok(()))
    }

    fn publish_with(
        &self,
        target: &Path,
        expected: Option<Sha256Digest>,
        bytes: &[u8],
        mut checkpoint: impl FnMut(Checkpoint) -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(target)?;
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
        if expected.is_none() {
            if !root
                .rename_exclusive(&temporary, &leaf)
                .map_err(|error| store_error(target, error))?
            {
                return Err(StoreError::Changed(target.to_path_buf()));
            }
        } else {
            root.rename_over(&temporary, &leaf)
                .map_err(|error| store_error(target, error))?;
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
