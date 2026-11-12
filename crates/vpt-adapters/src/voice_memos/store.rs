mod cloning;
mod errors;
use errors::{listing_error, recorder_error};
mod digest;
mod selection;
mod titles;

use crate::{Access, ContainedError, RootDir};
use std::fs::File;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::{Path, PathBuf};
use titles::Titles;
use vpt_application::ports::{
    Candidate, CloneError, CloneKind, RecorderError, RecorderStore, SourceMetadata, TitleLookup,
};
use vpt_domain::container::ReadFailure;
use vpt_domain::time::FileTime;

pub use selection::APPLE_SUBDIRECTORIES;

pub struct VoiceMemosStore {
    recordings: Option<RootDir>,
    titles: Option<Titles>,
}

impl VoiceMemosStore {
    pub fn with_titles(mut self, state_dir: PathBuf, refresh: bool) -> Self {
        self.titles = Some(Titles::from_path(state_dir, refresh));
        self
    }

    pub fn with_titles_root(mut self, state: &RootDir, refresh: bool) -> Self {
        self.titles = Some(Titles::from_root(state, refresh));
        self
    }

    pub fn open(recordings_dir: &Path) -> Result<Self, ContainedError> {
        let recordings = match RootDir::open(recordings_dir) {
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => None,
            opened => Some(opened?),
        };
        Ok(Self {
            recordings,
            titles: None,
        })
    }

    fn recordings(&self, path: &Path) -> Result<&RootDir, RecorderError> {
        self.recordings
            .as_ref()
            .ok_or_else(|| RecorderError::NotFound(path.into()))
    }
}

impl RecorderStore for VoiceMemosStore {
    type Handle = File;

    fn digest(
        &self,
        handle: &File,
        size: u64,
    ) -> Result<vpt_domain::digest::Sha256Digest, RecorderError> {
        digest::bounded(handle, size)
    }

    fn title(&self, file_name: &str) -> TitleLookup {
        self.titles
            .as_ref()
            .map_or(TitleLookup::Unavailable, |titles| titles.title(file_name))
    }

    fn refresh_titles(&self) -> Result<(), RecorderError> {
        match (&self.titles, &self.recordings) {
            (Some(titles), Some(recordings)) => titles.refresh(recordings),
            _ => Ok(()),
        }
    }

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        let recordings = self.recordings.as_ref().ok_or(RecorderError::Absent)?;
        let names = recordings
            .names()
            .map_err(|error| listing_error(recordings.path(), error))?;
        let mut candidates = Vec::new();
        for name in names {
            match selection::named(recordings, &name) {
                Ok(Some(candidate)) => candidates.push(candidate),
                Ok(None) | Err(RecorderError::NotRegular(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(candidates)
    }

    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        let recordings = self.recordings(path)?;
        let leaf = recordings
            .leaf(path)
            .map_err(|error| recorder_error(path, error))?;
        let name = leaf
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| RecorderError::Escape(path.into()))?;
        selection::named(recordings, name)?.ok_or_else(|| RecorderError::NotFound(path.into()))
    }

    fn open(&self, path: &Path) -> Result<File, RecorderError> {
        self.recordings(path)?
            .open_file(path, Access::Read)
            .map_err(|error| recorder_error(path, error))
    }

    fn metadata(&self, handle: &File) -> Result<SourceMetadata, RecorderError> {
        let metadata = handle
            .metadata()
            .map_err(|error| RecorderError::Io(error.kind().to_string()))?;
        Ok(SourceMetadata {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            mtime: FileTime {
                secs: metadata.mtime(),
                nanos: u32::try_from(metadata.mtime_nsec()).unwrap_or(0),
            },
        })
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        handle.read_exact_at(buf, offset).map_err(|_| ReadFailure)
    }

    fn clone_into(
        &self,
        handle: &File,
        directory: &Path,
        identity: (u64, u64),
        name: &str,
    ) -> Result<CloneKind, CloneError> {
        cloning::clone_into(handle, directory, identity, name)
    }

    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        selection::subdirectory_counts(self.recordings.as_ref())
    }
}

#[cfg(test)]
pub(super) mod tests;
