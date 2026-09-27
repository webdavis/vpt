mod publish;
mod staging;

use crate::{Access, ContainedError, Kind, RootDir};
use std::fs::File;
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

impl Archive for ClonefileArchive {
    type Handle = File;

    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        self.stage_with(clone)
    }

    fn open(&self, path: &Path) -> Result<File, ArchiveError> {
        self.root()?
            .open_file(path, Access::Read)
            .map_err(|error| archive_error(path, error))
    }

    fn size(&self, handle: &File) -> Result<u64, ArchiveError> {
        handle
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|error| io(&error))
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        handle.read_exact_at(buf, offset).map_err(|_| ReadFailure)
    }

    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError> {
        let root = self.root()?;
        root.regular(path)
            .map_err(|error| archive_error(path, error))?;
        root.revalidate()
            .map_err(|error| archive_error(&self.directory, error))
    }

    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError> {
        let mut file = self.open(path)?;
        crate::digest_open(&mut file).map_err(|error| io(&error))
    }

    fn target(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.ends_with(".m4a") && !name.starts_with('.'))
    }

    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.starts_with(STAGING_PREFIX))
    }

    fn publish(&self, staged: &Path, target_name: &str) -> Result<Published, ArchiveError> {
        publish::exclusive(self.root()?, staged, target_name)
    }

    fn sync_existing(&self, path: &Path) -> Result<(), ArchiveError> {
        publish::sync_file_and_directory(self.root()?, path)
    }
}

#[cfg(test)]
mod tests;
