mod port;
mod publish;
mod reader;
pub use reader::ArchiveReader;
mod staging;
mod verified;

use crate::{Access, ContainedError, Kind, RootDir};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vpt_application::ports::{
    Archive, ArchiveError, CloneError, CloneKind, Published, StageFailure, Staged,
};
use vpt_domain::container::ReadFailure;
use vpt_domain::digest::Sha256Digest;

pub const STAGING_PREFIX: &str = ".vpt-staging-";

pub struct ClonefileArchive {
    directory: PathBuf,
    root: Option<RootDir>,
    sequence: AtomicU64,
}

impl ClonefileArchive {
    pub fn open(audio_store: &Path) -> Result<Self, ContainedError> {
        Ok(Self {
            directory: audio_store.into(),
            root: Some(RootDir::open(audio_store)?),
            sequence: AtomicU64::new(0),
        })
    }

    pub fn open_read_only(audio_store: &Path) -> Result<Self, ContainedError> {
        match Self::open(audio_store) {
            Ok(archive) => Ok(archive),
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => Ok(Self {
                directory: audio_store.into(),
                root: None,
                sequence: AtomicU64::new(0),
            }),
            Err(error) => Err(error),
        }
    }

    fn root(&self) -> Result<&RootDir, ArchiveError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| ArchiveError::Io("the audio store is absent".into()))?;
        root.revalidate()
            .map_err(|error| archive_error(&self.directory, error))?;
        Ok(root)
    }

    fn list(&self, keep: impl Fn(&str) -> bool) -> Result<Vec<PathBuf>, ArchiveError> {
        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };
        root.revalidate()
            .map_err(|error| archive_error(&self.directory, error))?;
        let names = root
            .names()
            .map_err(|error| archive_error(&self.directory, error))?;
        let mut paths = Vec::new();
        for name in names.into_iter().filter(|name| keep(name)) {
            let path = root.path().join(&name);
            let stat = root
                .stat(Path::new(&name))
                .map_err(|error| archive_error(&path, error))?;
            if stat.kind == Kind::File {
                paths.push(path);
            }
        }
        root.revalidate()
            .map_err(|error| archive_error(&self.directory, error))?;
        Ok(paths)
    }
}

fn archive_error(path: &Path, error: ContainedError) -> ArchiveError {
    match error {
        ContainedError::Io {
            kind: std::io::ErrorKind::StorageFull,
            ..
        } => ArchiveError::NoSpace,
        ContainedError::Io { kind, .. } => ArchiveError::Io(kind.to_string()),
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => ArchiveError::Escape(path.into()),
    }
}

fn io(error: &std::io::Error) -> ArchiveError {
    if error.kind() == std::io::ErrorKind::StorageFull {
        ArchiveError::NoSpace
    } else {
        ArchiveError::Io(error.kind().to_string())
    }
}

#[cfg(test)]
mod tests;
