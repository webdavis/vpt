use super::{archive_error, io, verified};
use crate::{Access, RootDir};
use std::fs::{File, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use vpt_application::ports::ArchiveError;
use vpt_domain::digest::Sha256Digest;

#[derive(Debug)]
pub struct ArchiveReader {
    pub(super) file: File,
    pub(super) initial: Metadata,
    pub(super) path: PathBuf,
}

impl ArchiveReader {
    pub(super) fn open(root: &RootDir, path: &Path) -> Result<Self, ArchiveError> {
        let file = root
            .open_file(path, Access::Read)
            .map_err(|error| archive_error(path, error))?;
        let initial = file.metadata().map_err(|error| io(&error))?;
        Ok(Self {
            file,
            initial,
            path: path.into(),
        })
    }

    pub(super) fn digest(&self, root: &RootDir) -> Result<Sha256Digest, ArchiveError> {
        let mut file = self.file.try_clone().map_err(|error| io(&error))?;
        verified::digest_stable(root, &self.path, &mut file, &self.initial)
    }

    pub(super) fn verify_publication(
        &self,
        root: &RootDir,
        path: &Path,
        expected: &Sha256Digest,
    ) -> Result<(), ArchiveError> {
        let changed = || ArchiveError::Changed(path.into());
        let current = self.file.metadata().map_err(|_| changed())?;
        if current.len() != self.initial.len()
            || current.mtime() != self.initial.mtime()
            || current.mtime_nsec() != self.initial.mtime_nsec()
            || current.mode() & 0o7777 != 0o600
            || current.nlink() != 1
        {
            return Err(changed());
        }
        // Rename can change ctime; before rename the original metadata must still match.
        let before = if path == self.path {
            &self.initial
        } else {
            &current
        };
        let mut file = self.file.try_clone().map_err(|_| changed())?;
        let digest = verified::digest_stable(root, path, &mut file, before).map_err(|error| {
            if matches!(error, ArchiveError::Escape(_)) {
                error
            } else {
                changed()
            }
        })?;
        if digest != *expected {
            return Err(changed());
        }
        Ok(())
    }
}
