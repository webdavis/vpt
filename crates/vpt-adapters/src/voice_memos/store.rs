mod cloning;
mod titles;

use crate::{Access, ContainedError, Kind, RootDir};
use std::fs::File;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::{Path, PathBuf};
use titles::Titles;
use vpt_application::ports::{
    Candidate, CloneError, CloneKind, RecorderError, RecorderStore, SourceMetadata, TitleLookup,
};
use vpt_domain::container::ReadFailure;
use vpt_domain::time::FileTime;

pub const APPLE_SUBDIRECTORIES: [&str; 4] = [
    "Capture",
    "CaptureRecovery",
    "CloudRecordings_ckAssets",
    "EncryptedCloudRecordings",
];

pub struct VoiceMemosStore {
    recordings: RootDir,
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
        Ok(Self {
            recordings: RootDir::open(recordings_dir)?,
            titles: None,
        })
    }

    fn candidate_named(&self, name: &str) -> Result<Option<Candidate>, RecorderError> {
        if !name.ends_with(".m4a") {
            return Ok(None);
        }
        let path = self.recordings.path().join(name);
        let stat = self
            .recordings
            .stat(Path::new(name))
            .map_err(|error| recorder_error(&path, error))?;
        match stat.kind {
            Kind::File => Ok(Some(Candidate {
                path,
                file_name: name.into(),
                size: stat.size,
                mtime: FileTime {
                    secs: stat.mtime_secs,
                    nanos: stat.mtime_nanos,
                },
                flags: stat.flags,
            })),
            Kind::Link => Err(RecorderError::NotRegular(path)),
            Kind::Directory | Kind::Other => Ok(None),
        }
    }
}

impl RecorderStore for VoiceMemosStore {
    type Handle = File;

    fn title(&self, file_name: &str) -> TitleLookup {
        self.titles
            .as_ref()
            .map_or(TitleLookup::Unavailable, |titles| titles.title(file_name))
    }

    fn refresh_titles(&self) -> Result<(), RecorderError> {
        self.titles
            .as_ref()
            .map_or(Ok(()), |titles| titles.refresh(&self.recordings))
    }

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        let names = self
            .recordings
            .names()
            .map_err(|error| listing_error(self.recordings.path(), error))?;
        let mut candidates = Vec::new();
        for name in names {
            match self.candidate_named(&name) {
                Ok(Some(candidate)) => candidates.push(candidate),
                Ok(None) | Err(RecorderError::NotRegular(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(candidates)
    }

    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        let leaf = self
            .recordings
            .leaf(path)
            .map_err(|error| recorder_error(path, error))?;
        let name = leaf
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| RecorderError::Escape(path.into()))?;
        self.candidate_named(name)?
            .ok_or_else(|| RecorderError::NotFound(path.into()))
    }

    fn open(&self, path: &Path) -> Result<File, RecorderError> {
        self.recordings
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
        APPLE_SUBDIRECTORIES
            .iter()
            .map(|name| {
                let count = self
                    .recordings
                    .open_subdirectory(name)
                    .ok()
                    .and_then(|directory| directory.names().ok())
                    .map(|entries| entries.len() as u64);
                ((*name).into(), count)
            })
            .collect()
    }
}

fn recorder_error(path: &Path, error: ContainedError) -> RecorderError {
    match error {
        ContainedError::Escape { .. } | ContainedError::NotADirectory(_) => {
            RecorderError::Escape(path.into())
        }
        ContainedError::NotRegular(leaf) => RecorderError::NotRegular(leaf),
        ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        } => RecorderError::NotFound(path.into()),
        ContainedError::Io { kind, .. } => RecorderError::Io(kind.to_string()),
    }
}

fn listing_error(path: &Path, error: ContainedError) -> RecorderError {
    match error {
        ContainedError::Io { kind, .. } => RecorderError::Unreadable(kind.to_string()),
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => RecorderError::Escape(path.into()),
    }
}

#[cfg(test)]
pub(super) mod tests;
